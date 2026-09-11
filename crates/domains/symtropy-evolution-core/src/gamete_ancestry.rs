use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    chromosome_stochastic::parent_role_tag,
    AncestryAuthorityError, AncestryCopyId, ChromosomeId, ChromosomeMap, ChromosomeMapDigest,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileDigest, CrossoverParity,
    EvolutionError, HereditarySchema, HereditarySchemaDigest, LinkedGamete,
    LinkedGameteDerivationEvidence, LinkedGameteDerivationEvidenceDigest, LinkedGameteDigest,
    LocusId, ParentRole, PhasedAncestryState, PhasedAncestryStateDigest, PhasedHereditaryState,
    PhasedHereditaryStateDigest, ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
};

pub const MODELED_GAMETE_ANCESTRY_VERSION: u32 = 1;
pub const GAMETE_ANCESTRY_DERIVATION_VERSION: u32 = 1;

const MODELED_GAMETE_ANCESTRY_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:modeled-gamete-ancestry:v1\0";
const GAMETE_ANCESTRY_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:gamete-ancestry-provenance:v1\0";
const INITIAL_ANCESTRY_COPY_DRAW_DOMAIN: &[u8] =
    b"symtropy:evolution:gamete-ancestry:initial-copy:v1\0";

/// Persistent ancestry origin for one modeled hereditary locus in a haploid gamete.
///
/// This record says nothing about unmodeled sequence between loci and does not
/// establish a physical crossover breakpoint coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeledLocusAncestry {
    pub locus_id: LocusId,
    pub source_copy_id: AncestryCopyId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeGameteAncestry {
    pub chromosome_id: ChromosomeId,
    pub loci: Vec<ModeledLocusAncestry>,
}

/// Process-agnostic ancestry state for the modeled loci of one exact `LinkedGamete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeledGameteAncestry {
    pub state_version: u32,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub gamete_digest: LinkedGameteDigest,
    pub chromosomes: BTreeMap<ChromosomeId, ChromosomeGameteAncestry>,
}

impl ModeledGameteAncestry {
    fn new(
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        gamete: &LinkedGamete,
        chromosomes: impl IntoIterator<Item = ChromosomeGameteAncestry>,
    ) -> Result<Self, GameteAncestryError> {
        gamete.validate(schema, chromosome_map)?;
        let mut by_id = BTreeMap::new();
        for chromosome in chromosomes {
            let chromosome_id = chromosome.chromosome_id.clone();
            if by_id.insert(chromosome_id.clone(), chromosome).is_some() {
                return Err(GameteAncestryError::DuplicateChromosomeIdentity(
                    chromosome_id,
                ));
            }
        }
        let value = Self {
            state_version: MODELED_GAMETE_ANCESTRY_VERSION,
            schema_digest: schema.canonical_digest()?,
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            gamete_digest: gamete.canonical_digest(schema, chromosome_map)?,
            chromosomes: by_id,
        };
        value.validate_current(schema, chromosome_map, gamete)?;
        Ok(value)
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        gamete: &LinkedGamete,
    ) -> Result<(), GameteAncestryError> {
        gamete.validate(schema, chromosome_map)?;
        if self.state_version != MODELED_GAMETE_ANCESTRY_VERSION {
            return Err(GameteAncestryError::UnsupportedStateVersion(
                self.state_version,
            ));
        }
        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
            || self.gamete_digest != gamete.canonical_digest(schema, chromosome_map)?
        {
            return Err(GameteAncestryError::CurrentAuthorityMismatch);
        }
        if self.chromosomes.len() != chromosome_map.chromosomes.len() {
            return Err(GameteAncestryError::ChromosomeSetMismatch);
        }

        for (chromosome_id, definition) in &chromosome_map.chromosomes {
            let chromosome = self
                .chromosomes
                .get(chromosome_id)
                .ok_or(GameteAncestryError::ChromosomeSetMismatch)?;
            if chromosome.chromosome_id != *chromosome_id {
                return Err(GameteAncestryError::ChromosomeKeyMismatch {
                    key: chromosome_id.clone(),
                    value: chromosome.chromosome_id.clone(),
                });
            }
            if chromosome.loci.len() != definition.loci.len() {
                return Err(GameteAncestryError::LocusSequenceMismatch {
                    chromosome: chromosome_id.clone(),
                });
            }
            for (origin, mapped_locus) in chromosome.loci.iter().zip(&definition.loci) {
                if origin.locus_id != mapped_locus.locus_id {
                    return Err(GameteAncestryError::LocusSequenceMismatch {
                        chromosome: chromosome_id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        gamete: &LinkedGamete,
    ) -> Result<ModeledGameteAncestryDigest, GameteAncestryError> {
        self.validate_current(schema, chromosome_map, gamete)?;
        let mut digest = Sha256::new();
        digest.update(MODELED_GAMETE_ANCESTRY_DIGEST_DOMAIN);
        put_u32(&mut digest, self.state_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.gamete_digest.as_bytes());
        put_u64(&mut digest, self.chromosomes.len() as u64);
        for (chromosome_id, chromosome) in &self.chromosomes {
            put_text(&mut digest, chromosome_id.as_str());
            put_text(&mut digest, chromosome.chromosome_id.as_str());
            put_u64(&mut digest, chromosome.loci.len() as u64);
            for origin in &chromosome.loci {
                put_text(&mut digest, origin.locus_id.as_str());
                put_text(&mut digest, origin.source_copy_id.as_str());
            }
        }
        Ok(ModeledGameteAncestryDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModeledGameteAncestryDigest([u8; 32]);

impl ModeledGameteAncestryDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ModeledGameteAncestryDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ModeledGameteAncestryDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ModeledGameteAncestryDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameteAncestryDerivationProvenance {
    derivation_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    source_phased_state_digest: PhasedHereditaryStateDigest,
    source_ancestry_state_digest: PhasedAncestryStateDigest,
    recombination_profile_digest: ChromosomeRecombinationProfileDigest,
    gamete_evidence_digest: LinkedGameteDerivationEvidenceDigest,
    gamete_digest: LinkedGameteDigest,
    event_id: ReproductionEventId,
    parent_role: ParentRole,
    ancestry_digest: ModeledGameteAncestryDigest,
}

impl GameteAncestryDerivationProvenance {
    pub fn event_id(&self) -> &ReproductionEventId {
        &self.event_id
    }

    pub fn parent_role(&self) -> ParentRole {
        self.parent_role
    }

    pub fn ancestry_digest(&self) -> ModeledGameteAncestryDigest {
        self.ancestry_digest
    }

    pub fn canonical_digest(&self) -> GameteAncestryDerivationProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(GAMETE_ANCESTRY_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.derivation_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.source_phased_state_digest.as_bytes());
        digest.update(self.source_ancestry_state_digest.as_bytes());
        digest.update(self.recombination_profile_digest.as_bytes());
        digest.update(self.gamete_evidence_digest.as_bytes());
        digest.update(self.gamete_digest.as_bytes());
        put_text(&mut digest, self.event_id.as_str());
        digest.update([parent_role_tag(self.parent_role)]);
        digest.update(self.ancestry_digest.as_bytes());
        GameteAncestryDerivationProvenanceDigest(digest.finalize().into())
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        source: &PhasedHereditaryState,
        source_ancestry: &PhasedAncestryState,
        profile: &ChromosomeRecombinationProfile,
        gamete_evidence: &LinkedGameteDerivationEvidence,
        event: &ReproductionEventId,
        parent_role: ParentRole,
        ancestry: &ModeledGameteAncestry,
    ) -> Result<(), GameteAncestryError> {
        if self.derivation_version != GAMETE_ANCESTRY_DERIVATION_VERSION {
            return Err(GameteAncestryError::UnsupportedDerivationVersion(
                self.derivation_version,
            ));
        }
        if &self.event_id != event || self.parent_role != parent_role {
            return Err(GameteAncestryError::EventContextMismatch);
        }
        source.validate(schema, chromosome_map)?;
        source_ancestry.validate_current(schema, chromosome_map, source)?;
        profile.validate(schema, chromosome_map)?;
        gamete_evidence.validate_current(
            schema,
            chromosome_map,
            source,
            profile,
            event,
            parent_role,
        )?;
        let gamete = gamete_evidence.gamete();
        ancestry.validate_current(schema, chromosome_map, gamete)?;

        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
            || self.source_phased_state_digest != source.canonical_digest(schema, chromosome_map)?
            || self.source_ancestry_state_digest
                != source_ancestry.canonical_digest(schema, chromosome_map, source)?
            || self.recombination_profile_digest != profile.canonical_digest(schema, chromosome_map)?
            || self.gamete_evidence_digest != gamete_evidence.canonical_digest()
            || self.gamete_digest != gamete.canonical_digest(schema, chromosome_map)?
            || self.ancestry_digest != ancestry.canonical_digest(schema, chromosome_map, gamete)?
        {
            return Err(GameteAncestryError::CurrentAuthorityMismatch);
        }

        let recomputed = derive_modeled_gamete_ancestry(
            schema,
            chromosome_map,
            source,
            source_ancestry,
            profile,
            gamete_evidence,
            event,
            parent_role,
        )?;
        if recomputed.ancestry != *ancestry || recomputed.provenance != *self {
            return Err(GameteAncestryError::DerivationMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameteAncestryDerivationProvenanceDigest([u8; 32]);

impl GameteAncestryDerivationProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for GameteAncestryDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GameteAncestryDerivationProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for GameteAncestryDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameteAncestryDerivation {
    pub ancestry: ModeledGameteAncestry,
    pub provenance: GameteAncestryDerivationProvenance,
}

pub fn derive_modeled_gamete_ancestry(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    source_ancestry: &PhasedAncestryState,
    profile: &ChromosomeRecombinationProfile,
    gamete_evidence: &LinkedGameteDerivationEvidence,
    event: &ReproductionEventId,
    parent_role: ParentRole,
) -> Result<GameteAncestryDerivation, GameteAncestryError> {
    if schema.ploidy != 2 {
        return Err(GameteAncestryError::RequiresDiploid(schema.ploidy));
    }
    source.validate(schema, chromosome_map)?;
    source_ancestry.validate_current(schema, chromosome_map, source)?;
    profile.validate(schema, chromosome_map)?;
    gamete_evidence.validate_current(
        schema,
        chromosome_map,
        source,
        profile,
        event,
        parent_role,
    )?;

    let gamete = gamete_evidence.gamete();
    let mut chromosomes = Vec::with_capacity(chromosome_map.chromosomes.len());

    for (chromosome_id, definition) in &chromosome_map.chromosomes {
        let origins = match gamete_evidence {
            LinkedGameteDerivationEvidence::ZeroCrossover(derivation) => {
                let segment = derivation
                    .provenance
                    .segments()
                    .get(chromosome_id)
                    .ok_or(GameteAncestryError::ChromosomeSetMismatch)?;
                let copy_id = initial_ancestry_copy(
                    schema,
                    chromosome_map,
                    source,
                    source_ancestry,
                    event,
                    parent_role,
                    chromosome_id,
                    usize::from(segment.source_haplotype_slot),
                )?;
                definition
                    .loci
                    .iter()
                    .map(|mapped_locus| ModeledLocusAncestry {
                        locus_id: mapped_locus.locus_id.clone(),
                        source_copy_id: copy_id.clone(),
                    })
                    .collect()
            }
            LinkedGameteDerivationEvidence::MarkerMarginalPoisson(derivation) => {
                let marker = derivation
                    .provenance
                    .chromosomes()
                    .get(chromosome_id)
                    .ok_or(GameteAncestryError::ChromosomeSetMismatch)?;
                if marker.adjacent_intervals.len() + 1 != definition.loci.len() {
                    return Err(GameteAncestryError::LocusSequenceMismatch {
                        chromosome: chromosome_id.clone(),
                    });
                }
                let initial_slot = usize::from(marker.initial_source_haplotype_slot);
                let initial_class = source_ancestry.copy_ids_for_haplotype_content_at_slot(
                    schema,
                    chromosome_map,
                    source,
                    chromosome_id,
                    initial_slot,
                )?;
                let ambiguous_identical = initial_class.len() == 2;
                if !matches!(initial_class.len(), 1 | 2) {
                    return Err(GameteAncestryError::UnexpectedAncestryClassCardinality {
                        chromosome: chromosome_id.clone(),
                        observed: initial_class.len(),
                    });
                }
                let mut current = choose_from_class(
                    initial_class,
                    event,
                    parent_role,
                    chromosome_id,
                )?;
                let mut loci = Vec::with_capacity(definition.loci.len());
                loci.push(ModeledLocusAncestry {
                    locus_id: definition.loci[0].locus_id.clone(),
                    source_copy_id: current.clone(),
                });

                for (index, interval) in marker.adjacent_intervals.iter().enumerate() {
                    let left = &definition.loci[index];
                    let right = &definition.loci[index + 1];
                    if interval.left_locus_id != left.locus_id
                        || interval.right_locus_id != right.locus_id
                    {
                        return Err(GameteAncestryError::LocusSequenceMismatch {
                            chromosome: chromosome_id.clone(),
                        });
                    }

                    if ambiguous_identical {
                        if interval.parity == CrossoverParity::Odd {
                            current = toggle_ambiguous_copy(initial_class, &current, chromosome_id)?;
                        }
                    } else {
                        let class = source_ancestry.copy_ids_for_haplotype_content_at_slot(
                            schema,
                            chromosome_map,
                            source,
                            chromosome_id,
                            usize::from(interval.source_haplotype_slot_after),
                        )?;
                        if class.len() != 1 {
                            return Err(GameteAncestryError::UnexpectedAncestryClassCardinality {
                                chromosome: chromosome_id.clone(),
                                observed: class.len(),
                            });
                        }
                        current = class[0].clone();
                    }

                    loci.push(ModeledLocusAncestry {
                        locus_id: right.locus_id.clone(),
                        source_copy_id: current.clone(),
                    });
                }
                loci
            }
        };

        chromosomes.push(ChromosomeGameteAncestry {
            chromosome_id: chromosome_id.clone(),
            loci: origins,
        });
    }

    let ancestry = ModeledGameteAncestry::new(schema, chromosome_map, gamete, chromosomes)?;
    let provenance = GameteAncestryDerivationProvenance {
        derivation_version: GAMETE_ANCESTRY_DERIVATION_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        source_phased_state_digest: source.canonical_digest(schema, chromosome_map)?,
        source_ancestry_state_digest: source_ancestry
            .canonical_digest(schema, chromosome_map, source)?,
        recombination_profile_digest: profile.canonical_digest(schema, chromosome_map)?,
        gamete_evidence_digest: gamete_evidence.canonical_digest(),
        gamete_digest: gamete.canonical_digest(schema, chromosome_map)?,
        event_id: event.clone(),
        parent_role,
        ancestry_digest: ancestry.canonical_digest(schema, chromosome_map, gamete)?,
    };

    Ok(GameteAncestryDerivation {
        ancestry,
        provenance,
    })
}

fn initial_ancestry_copy(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    source_ancestry: &PhasedAncestryState,
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
    haplotype_slot: usize,
) -> Result<AncestryCopyId, GameteAncestryError> {
    let class = source_ancestry.copy_ids_for_haplotype_content_at_slot(
        schema,
        chromosome_map,
        source,
        chromosome_id,
        haplotype_slot,
    )?;
    choose_from_class(class, event, parent_role, chromosome_id)
}

fn choose_from_class(
    class: &[AncestryCopyId],
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
) -> Result<AncestryCopyId, GameteAncestryError> {
    match class {
        [only] => Ok(only.clone()),
        [first, second] => {
            let index = semantic_initial_ancestry_copy_index(event, parent_role, chromosome_id);
            Ok(if index == 0 {
                first.clone()
            } else {
                second.clone()
            })
        }
        other => Err(GameteAncestryError::UnexpectedAncestryClassCardinality {
            chromosome: chromosome_id.clone(),
            observed: other.len(),
        }),
    }
}

fn toggle_ambiguous_copy(
    class: &[AncestryCopyId],
    current: &AncestryCopyId,
    chromosome_id: &ChromosomeId,
) -> Result<AncestryCopyId, GameteAncestryError> {
    match class {
        [first, second] if current == first => Ok(second.clone()),
        [first, second] if current == second => Ok(first.clone()),
        [_, _] => Err(GameteAncestryError::DerivationMismatch),
        other => Err(GameteAncestryError::UnexpectedAncestryClassCardinality {
            chromosome: chromosome_id.clone(),
            observed: other.len(),
        }),
    }
}

fn semantic_initial_ancestry_copy_index(
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
) -> usize {
    let mut digest = Sha256::new();
    digest.update(INITIAL_ANCESTRY_COPY_DRAW_DOMAIN);
    put_text(&mut digest, event.as_str());
    digest.update([parent_role_tag(parent_role)]);
    put_text(&mut digest, chromosome_id.as_str());
    let bytes: [u8; 32] = digest.finalize().into();
    usize::from(bytes[0] & 1)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameteAncestryError {
    Evolution(EvolutionError),
    Ancestry(AncestryAuthorityError),
    UnsupportedStateVersion(u32),
    UnsupportedDerivationVersion(u32),
    RequiresDiploid(u8),
    CurrentAuthorityMismatch,
    EventContextMismatch,
    ChromosomeSetMismatch,
    DuplicateChromosomeIdentity(ChromosomeId),
    ChromosomeKeyMismatch {
        key: ChromosomeId,
        value: ChromosomeId,
    },
    LocusSequenceMismatch {
        chromosome: ChromosomeId,
    },
    UnexpectedAncestryClassCardinality {
        chromosome: ChromosomeId,
        observed: usize,
    },
    DerivationMismatch,
}

impl From<EvolutionError> for GameteAncestryError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<AncestryAuthorityError> for GameteAncestryError {
    fn from(value: AncestryAuthorityError) -> Self {
        Self::Ancestry(value)
    }
}

impl fmt::Display for GameteAncestryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Ancestry(error) => write!(f, "ancestry authority error: {error}"),
            Self::UnsupportedStateVersion(version) => {
                write!(f, "unsupported modeled-gamete-ancestry version {version}")
            }
            Self::UnsupportedDerivationVersion(version) => {
                write!(f, "unsupported gamete-ancestry derivation version {version}")
            }
            Self::RequiresDiploid(ploidy) => {
                write!(f, "gamete ancestry v1 requires diploid source state, observed ploidy {ploidy}")
            }
            Self::CurrentAuthorityMismatch => {
                write!(f, "gamete ancestry does not match exact current authority")
            }
            Self::EventContextMismatch => {
                write!(f, "gamete ancestry event or parent-role context mismatch")
            }
            Self::ChromosomeSetMismatch => write!(f, "gamete ancestry chromosome set mismatch"),
            Self::DuplicateChromosomeIdentity(chromosome) => write!(
                f,
                "duplicate gamete ancestry chromosome {}",
                chromosome.as_str()
            ),
            Self::ChromosomeKeyMismatch { key, value } => write!(
                f,
                "gamete ancestry chromosome key {} does not match embedded chromosome {}",
                key.as_str(),
                value.as_str()
            ),
            Self::LocusSequenceMismatch { chromosome } => write!(
                f,
                "gamete ancestry modeled-locus sequence does not match chromosome {}",
                chromosome.as_str()
            ),
            Self::UnexpectedAncestryClassCardinality {
                chromosome,
                observed,
            } => write!(
                f,
                "chromosome {} ancestry content class has unsupported diploid cardinality {}",
                chromosome.as_str(),
                observed
            ),
            Self::DerivationMismatch => write!(f, "gamete ancestry deterministic replay mismatch"),
        }
    }
}

impl Error for GameteAncestryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            Self::Ancestry(error) => Some(error),
            _ => None,
        }
    }
}
