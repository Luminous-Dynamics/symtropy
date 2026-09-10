use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    EvolutionError, EvolutionOperatorProfile, EvolutionOperatorProfileDigest,
    HereditarySchema, HereditarySchemaDigest, HereditaryState, HereditaryStateDigest,
    LocusId, ReproductionEventId, PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const REPRODUCTION_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:reproduction-provenance:v1\0";
const RNG_DOMAIN: &[u8] = b"symtropy:evolution:semantic-draw:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductionMode {
    Clonal,
    BiparentalDiploidIndependentLoci,
}

impl ReproductionMode {
    fn tag(self) -> u8 {
        match self {
            Self::Clonal => 0,
            Self::BiparentalDiploidIndependentLoci => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParentRole {
    ClonalParent,
    ParentA,
    ParentB,
}

impl ParentRole {
    fn tag(self) -> u8 {
        match self {
            Self::ClonalParent => 0,
            Self::ParentA => 1,
            Self::ParentB => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentHereditaryRef {
    pub role: ParentRole,
    pub hereditary_digest: HereditaryStateDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductionProvenance {
    schema_digest: HereditarySchemaDigest,
    event_id: ReproductionEventId,
    mode: ReproductionMode,
    operator_digest: EvolutionOperatorProfileDigest,
    parents: Vec<ParentHereditaryRef>,
    child_digest: HereditaryStateDigest,
}

impl ReproductionProvenance {
    pub fn schema_digest(&self) -> HereditarySchemaDigest {
        self.schema_digest
    }

    pub fn event_id(&self) -> &ReproductionEventId {
        &self.event_id
    }

    pub fn mode(&self) -> ReproductionMode {
        self.mode
    }

    pub fn operator_digest(&self) -> EvolutionOperatorProfileDigest {
        self.operator_digest
    }

    pub fn parents(&self) -> &[ParentHereditaryRef] {
        &self.parents
    }

    pub fn child_digest(&self) -> HereditaryStateDigest {
        self.child_digest
    }

    pub fn canonical_digest(&self) -> ReproductionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(REPRODUCTION_DIGEST_DOMAIN);
        digest.update(self.schema_digest.as_bytes());
        put_text(&mut digest, self.event_id.as_str());
        digest.update([self.mode.tag()]);
        digest.update(self.operator_digest.as_bytes());
        put_u64(&mut digest, self.parents.len() as u64);
        for parent in &self.parents {
            digest.update([parent.role.tag()]);
            digest.update(parent.hereditary_digest.as_bytes());
        }
        digest.update(self.child_digest.as_bytes());
        ReproductionProvenanceDigest(digest.finalize().into())
    }

    /// Revalidate restored provenance against exact current biological inputs.
    /// Deserialization alone never makes a parentage claim authoritative.
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        parents: &[&HereditaryState],
        operators: &EvolutionOperatorProfile,
        child: &HereditaryState,
    ) -> Result<(), EvolutionError> {
        if schema.canonical_digest()? != self.schema_digest {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if operators.canonical_digest()? != self.operator_digest {
            return Err(EvolutionError::OperatorAuthorityMismatch);
        }
        for parent in parents {
            parent.validate(schema)?;
        }
        child.validate(schema)?;

        if parent_refs(schema, parents, self.mode)? != self.parents {
            return Err(EvolutionError::ParentageMismatch);
        }
        if child.canonical_digest(schema)? != self.child_digest {
            return Err(EvolutionError::ChildDigestMismatch);
        }

        let recomputed = derive_child_content(
            schema,
            parents,
            &self.event_id,
            operators,
            self.mode,
        )?;
        if recomputed != *child {
            return Err(EvolutionError::ChildDerivationMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReproductionProvenanceDigest([u8; 32]);

impl ReproductionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ReproductionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReproductionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ReproductionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffspringDerivation {
    pub child: HereditaryState,
    pub provenance: ReproductionProvenance,
}

pub fn derive_offspring(
    schema: &HereditarySchema,
    parents: &[&HereditaryState],
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    mode: ReproductionMode,
) -> Result<OffspringDerivation, EvolutionError> {
    schema.validate()?;
    operators.validate()?;
    for parent in parents {
        parent.validate(schema)?;
    }

    let child = derive_child_content(schema, parents, event, operators, mode)?;
    let provenance = ReproductionProvenance {
        schema_digest: schema.canonical_digest()?,
        event_id: event.clone(),
        mode,
        operator_digest: operators.canonical_digest()?,
        parents: parent_refs(schema, parents, mode)?,
        child_digest: child.canonical_digest(schema)?,
    };
    provenance.validate_current(schema, parents, operators, &child)?;

    Ok(OffspringDerivation { child, provenance })
}

fn parent_refs(
    schema: &HereditarySchema,
    parents: &[&HereditaryState],
    mode: ReproductionMode,
) -> Result<Vec<ParentHereditaryRef>, EvolutionError> {
    match mode {
        ReproductionMode::Clonal => {
            if parents.len() != 1 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 1,
                    observed: parents.len(),
                });
            }
            Ok(vec![ParentHereditaryRef {
                role: ParentRole::ClonalParent,
                hereditary_digest: parents[0].canonical_digest(schema)?,
            }])
        }
        ReproductionMode::BiparentalDiploidIndependentLoci => {
            if parents.len() != 2 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 2,
                    observed: parents.len(),
                });
            }
            Ok(vec![
                ParentHereditaryRef {
                    role: ParentRole::ParentA,
                    hereditary_digest: parents[0].canonical_digest(schema)?,
                },
                ParentHereditaryRef {
                    role: ParentRole::ParentB,
                    hereditary_digest: parents[1].canonical_digest(schema)?,
                },
            ])
        }
    }
}

fn derive_child_content(
    schema: &HereditarySchema,
    parents: &[&HereditaryState],
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    mode: ReproductionMode,
) -> Result<HereditaryState, EvolutionError> {
    let mut child = BTreeMap::new();

    match mode {
        ReproductionMode::Clonal => {
            if parents.len() != 1 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 1,
                    observed: parents.len(),
                });
            }
            for locus_id in schema.loci.keys() {
                let mut copies = parents[0]
                    .copies
                    .get(locus_id)
                    .expect("validated parent has every schema locus")
                    .clone();
                mutate_copies(schema, locus_id, &mut copies, event, operators)?;
                child.insert(locus_id.clone(), copies);
            }
        }
        ReproductionMode::BiparentalDiploidIndependentLoci => {
            if parents.len() != 2 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 2,
                    observed: parents.len(),
                });
            }
            if schema.ploidy != 2 {
                return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
            }

            for locus_id in schema.loci.keys() {
                let parent_a = parents[0]
                    .copies
                    .get(locus_id)
                    .expect("validated parent has every schema locus");
                let parent_b = parents[1]
                    .copies
                    .get(locus_id)
                    .expect("validated parent has every schema locus");

                let a_index = draw_below(
                    event,
                    operators,
                    locus_id,
                    0,
                    DrawKind::Recombination,
                    "parent-a-copy",
                    parent_a.len() as u64,
                )? as usize;
                let b_index = draw_below(
                    event,
                    operators,
                    locus_id,
                    1,
                    DrawKind::Recombination,
                    "parent-b-copy",
                    parent_b.len() as u64,
                )? as usize;

                let mut copies = vec![parent_a[a_index].clone(), parent_b[b_index].clone()];
                mutate_copies(schema, locus_id, &mut copies, event, operators)?;
                child.insert(locus_id.clone(), copies);
            }
        }
    }

    HereditaryState::new(schema, child)
}

fn mutate_copies(
    schema: &HereditarySchema,
    locus_id: &LocusId,
    copies: &mut [crate::AlleleId],
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
) -> Result<(), EvolutionError> {
    let locus = schema
        .loci
        .get(locus_id)
        .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;

    for (copy_index, allele) in copies.iter_mut().enumerate() {
        let draw = draw_below(
            event,
            operators,
            locus_id,
            copy_index as u64,
            DrawKind::Mutation,
            "occurs",
            u64::from(PROBABILITY_SCALE_PPM),
        )?;
        if draw >= u64::from(operators.mutation.per_copy_rate_ppm)
            || locus.allowed_alleles.len() <= 1
        {
            continue;
        }

        let alternatives: Vec<_> = locus
            .allowed_alleles
            .iter()
            .filter(|candidate| *candidate != allele)
            .cloned()
            .collect();
        let choice = draw_below(
            event,
            operators,
            locus_id,
            copy_index as u64,
            DrawKind::Mutation,
            "alternate",
            alternatives.len() as u64,
        )? as usize;
        *allele = alternatives[choice].clone();
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum DrawKind {
    Mutation,
    Recombination,
}

fn draw_below(
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    locus: &LocusId,
    copy_index: u64,
    kind: DrawKind,
    purpose: &str,
    upper: u64,
) -> Result<u64, EvolutionError> {
    if upper == 0 {
        return Err(EvolutionError::InvalidDrawUpperBound);
    }
    if upper == 1 {
        return Ok(0);
    }

    let zone = u64::MAX - (u64::MAX % upper);
    let mut attempt = 0_u64;
    loop {
        let value = semantic_draw_u64(
            event,
            operators,
            locus,
            copy_index,
            kind,
            purpose,
            attempt,
        )?;
        if value < zone {
            return Ok(value % upper);
        }
        attempt = attempt
            .checked_add(1)
            .ok_or(EvolutionError::CountOverflow)?;
    }
}

fn semantic_draw_u64(
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    locus: &LocusId,
    copy_index: u64,
    kind: DrawKind,
    purpose: &str,
    attempt: u64,
) -> Result<u64, EvolutionError> {
    operators.validate()?;
    let mut digest = Sha256::new();
    digest.update(RNG_DOMAIN);
    put_text(&mut digest, event.as_str());
    put_text(&mut digest, operators.profile_id.as_str());
    put_text(&mut digest, &operators.version);
    match kind {
        DrawKind::Mutation => operators.mutation.put_randomness_identity(&mut digest),
        DrawKind::Recombination => operators.recombination.put_randomness_identity(&mut digest),
    }
    put_text(&mut digest, locus.as_str());
    put_u64(&mut digest, copy_index);
    put_text(&mut digest, purpose);
    put_u64(&mut digest, attempt);
    let bytes: [u8; 32] = digest.finalize().into();
    Ok(u64::from_le_bytes(
        bytes[..8]
            .try_into()
            .expect("SHA-256 output has an 8-byte prefix"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AlleleId, HereditarySchemaId, LocusDefinition, MutationProfile,
        OperatorProfileId, RecombinationMode, RecombinationProfile,
    };

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn locus(id: &str, alleles: &[&str]) -> LocusDefinition {
        LocusDefinition::new(
            LocusId::new(id).unwrap(),
            alleles.iter().map(|id| allele(id)),
        )
        .unwrap()
    }

    fn schema(loci: Vec<LocusDefinition>) -> HereditarySchema {
        HereditarySchema::new(HereditarySchemaId::new("diploid-v0").unwrap(), 2, loci).unwrap()
    }

    fn state(schema: &HereditarySchema, values: &[(&str, &[&str])]) -> HereditaryState {
        let copies = values
            .iter()
            .map(|(locus, alleles)| {
                (
                    LocusId::new(*locus).unwrap(),
                    alleles.iter().map(|id| allele(id)).collect(),
                )
            })
            .collect();
        HereditaryState::new(schema, copies).unwrap()
    }

    fn operators(rate: u32) -> EvolutionOperatorProfile {
        EvolutionOperatorProfile {
            profile_id: OperatorProfileId::new("evo-v0").unwrap(),
            version: "v1".into(),
            mutation: MutationProfile {
                model_id: "point-substitution".into(),
                version: "v1".into(),
                per_copy_rate_ppm: rate,
            },
            recombination: RecombinationProfile {
                model_id: "independent-loci".into(),
                version: "v1".into(),
                mode: RecombinationMode::IndependentLoci,
            },
        }
    }

    #[test]
    fn replay_and_provenance_are_exact() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let parent = state(&schema, &[("pigment", &["dark", "light"])]);
        let event = ReproductionEventId::new("birth-42").unwrap();
        let operators = operators(25_000);

        let first = derive_offspring(
            &schema,
            &[&parent],
            &event,
            &operators,
            ReproductionMode::Clonal,
        )
        .unwrap();
        let second = derive_offspring(
            &schema,
            &[&parent],
            &event,
            &operators,
            ReproductionMode::Clonal,
        )
        .unwrap();

        assert_eq!(first, second);
        first
            .provenance
            .validate_current(&schema, &[&parent], &operators, &first.child)
            .unwrap();
        assert_eq!(
            first.provenance.canonical_digest(),
            second.provenance.canonical_digest()
        );
    }

    #[test]
    fn changed_parent_content_invalidates_parentage() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let original = state(&schema, &[("pigment", &["dark", "light"])]);
        let replacement = state(&schema, &[("pigment", &["light", "light"])]);
        let event = ReproductionEventId::new("parentage-check").unwrap();
        let operators = operators(0);
        let derivation = derive_offspring(
            &schema,
            &[&original],
            &event,
            &operators,
            ReproductionMode::Clonal,
        )
        .unwrap();

        assert_eq!(
            derivation.provenance.validate_current(
                &schema,
                &[&replacement],
                &operators,
                &derivation.child,
            ),
            Err(EvolutionError::ParentageMismatch)
        );
    }

    #[test]
    fn parent_roles_are_provenance_significant() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let a = state(&schema, &[("pigment", &["dark", "dark"])]);
        let b = state(&schema, &[("pigment", &["light", "light"])]);
        let event = ReproductionEventId::new("role-test").unwrap();
        let operators = operators(0);

        let ab = derive_offspring(
            &schema,
            &[&a, &b],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .unwrap();
        let ba = derive_offspring(
            &schema,
            &[&b, &a],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .unwrap();

        assert_ne!(
            ab.provenance.canonical_digest(),
            ba.provenance.canonical_digest()
        );
    }

    #[test]
    fn mutation_rate_change_does_not_reroll_mutation_variate() {
        let event = ReproductionEventId::new("rate-sweep").unwrap();
        let locus = LocusId::new("pigment").unwrap();
        let low = operators(1_000);
        let high = operators(900_000);
        let low_draw = semantic_draw_u64(
            &event,
            &low,
            &locus,
            0,
            DrawKind::Mutation,
            "occurs",
            0,
        )
        .unwrap();
        let high_draw = semantic_draw_u64(
            &event,
            &high,
            &locus,
            0,
            DrawKind::Mutation,
            "occurs",
            0,
        )
        .unwrap();
        assert_eq!(low_draw, high_draw);
        assert_ne!(low.canonical_digest().unwrap(), high.canonical_digest().unwrap());
    }

    #[test]
    fn unrelated_locus_does_not_shift_existing_locus() {
        let schema_a = schema(vec![locus("pigment", &["dark", "light"])]);
        let schema_ab = schema(vec![
            locus("pigment", &["dark", "light"]),
            locus("enzyme", &["slow", "fast"]),
        ]);
        let a1 = state(&schema_a, &[("pigment", &["dark", "light"])]);
        let a2 = state(&schema_a, &[("pigment", &["light", "light"])]);
        let ab1 = state(
            &schema_ab,
            &[("pigment", &["dark", "light"]), ("enzyme", &["slow", "fast"])],
        );
        let ab2 = state(
            &schema_ab,
            &[("pigment", &["light", "light"]), ("enzyme", &["fast", "fast"])],
        );
        let event = ReproductionEventId::new("keyed-loci").unwrap();
        let operators = operators(0);

        let child_a = derive_offspring(
            &schema_a,
            &[&a1, &a2],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .unwrap();
        let child_ab = derive_offspring(
            &schema_ab,
            &[&ab1, &ab2],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .unwrap();
        let pigment = LocusId::new("pigment").unwrap();
        assert_eq!(
            child_a.child.copies.get(&pigment),
            child_ab.child.copies.get(&pigment)
        );
    }
}
