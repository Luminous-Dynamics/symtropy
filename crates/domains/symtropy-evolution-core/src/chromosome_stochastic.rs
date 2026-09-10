use crate::{
    canonical::{put_text, put_u64},
    ChromosomeId, EvolutionError, LocusId, ParentRole, ReproductionEventId,
};
use sha2::{Digest, Sha256};

// Frozen C3B1 domain. Do not change these bytes: zero-crossover histories depend on them.
const INITIAL_HAPLOTYPE_DRAW_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-gamete:no-crossovers-independent-assortment:v1\0";

const MARKER_PARITY_DRAW_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-gamete:marker-parity-poisson-no-interference:v1\0";

pub(crate) fn parent_role_tag(parent_role: ParentRole) -> u8 {
    match parent_role {
        ParentRole::ClonalParent => 0,
        ParentRole::ParentA => 1,
        ParentRole::ParentB => 2,
    }
}

/// Frozen C3B1 whole-homolog opportunity field.
pub(crate) fn semantic_initial_haplotype_slot(
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
) -> usize {
    let mut digest = Sha256::new();
    digest.update(INITIAL_HAPLOTYPE_DRAW_DOMAIN);
    put_text(&mut digest, event.as_str());
    digest.update([parent_role_tag(parent_role)]);
    put_text(&mut digest, chromosome_id.as_str());
    let bytes: [u8; 32] = digest.finalize().into();
    usize::from(bytes[0] & 1)
}

/// Unbiased semantic integer in `[0, 1_000_000)` for one adjacent marker interval.
///
/// Genetic-map distance is deliberately absent from the stochastic key. A map-distance
/// counterfactual changes the deterministic threshold/authority while preserving the
/// underlying opportunity draw for this exact event/role/chromosome/locus pair.
pub(crate) fn semantic_marker_parity_draw_ppm(
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
    left_locus: &LocusId,
    right_locus: &LocusId,
) -> Result<u32, EvolutionError> {
    const UPPER: u64 = 1_000_000;
    let value = semantic_u64_below(
        MARKER_PARITY_DRAW_DOMAIN,
        event,
        parent_role,
        chromosome_id,
        left_locus,
        right_locus,
        UPPER,
    )?;
    u32::try_from(value).map_err(|_| EvolutionError::SamplingInvariantViolation)
}

fn semantic_u64_below(
    domain: &[u8],
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
    left_locus: &LocusId,
    right_locus: &LocusId,
    upper: u64,
) -> Result<u64, EvolutionError> {
    if upper == 0 {
        return Err(EvolutionError::InvalidDrawUpperBound);
    }
    if upper == 1 {
        return Ok(0);
    }

    // Accept only a prefix whose size is divisible by `upper`. Retry the tiny tail
    // with an explicit semantic attempt coordinate rather than introducing mutable RNG.
    let zone = u64::MAX - (u64::MAX % upper);
    let mut attempt = 0_u64;
    loop {
        let mut digest = Sha256::new();
        digest.update(domain);
        put_text(&mut digest, event.as_str());
        digest.update([parent_role_tag(parent_role)]);
        put_text(&mut digest, chromosome_id.as_str());
        put_text(&mut digest, left_locus.as_str());
        put_text(&mut digest, right_locus.as_str());
        put_u64(&mut digest, attempt);
        let bytes: [u8; 32] = digest.finalize().into();
        let value = u64::from_le_bytes(
            bytes[..8]
                .try_into()
                .map_err(|_| EvolutionError::SamplingInvariantViolation)?,
        );
        if value < zone {
            return Ok(value % upper);
        }
        attempt = attempt
            .checked_add(1)
            .ok_or(EvolutionError::CountOverflow)?;
    }
}
