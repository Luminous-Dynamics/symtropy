use crate::{
    AlleleId, EvolutionError, HereditarySchema, LocusId, PopulationGeneticState, PopulationId,
};
use std::collections::BTreeMap;

/// Select an arbitrary number of marginal allele copies without replacement at
/// every locus.
///
/// The caller owns the stochastic coordinates and supplies a full-width
/// priority for each conceptual source allele copy. This helper owns only the
/// exact copy-selection mechanics. `target_copies` may be zero and need not be
/// divisible by ploidy; this is useful for census-preserving admixture where the
/// replaced copy count is a quantized fraction of the destination gene pool.
pub(crate) fn sample_marginal_copy_counts_without_replacement<F>(
    schema: &HereditarySchema,
    source: &PopulationGeneticState,
    target_copies: u64,
    mut priority_for: F,
) -> Result<BTreeMap<LocusId, BTreeMap<AlleleId, u64>>, EvolutionError>
where
    F: FnMut(&LocusId, &AlleleId, u64) -> [u8; 32],
{
    source.validate(schema)?;
    let available_copies = source
        .census_individuals
        .checked_mul(u64::from(schema.ploidy))
        .ok_or(EvolutionError::CountOverflow)?;
    if target_copies > available_copies {
        return Err(EvolutionError::SamplingInvariantViolation);
    }
    let target_len = usize::try_from(target_copies).map_err(|_| EvolutionError::CountOverflow)?;
    let mut selected_counts = BTreeMap::new();

    for locus_id in schema.loci.keys() {
        let source_counts = source
            .allele_copy_counts
            .get(locus_id)
            .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
        let mut candidates = Vec::new();
        for (allele_id, count) in source_counts {
            for within_allele_ordinal in 0..*count {
                candidates.push((
                    priority_for(locus_id, allele_id, within_allele_ordinal),
                    allele_id.clone(),
                    within_allele_ordinal,
                ));
            }
        }
        candidates.sort();
        if target_len > candidates.len() {
            return Err(EvolutionError::SamplingInvariantViolation);
        }

        let mut counts: BTreeMap<AlleleId, u64> = BTreeMap::new();
        for (_, allele, _) in candidates.into_iter().take(target_len) {
            let count = counts.entry(allele).or_insert(0);
            *count = count.checked_add(1).ok_or(EvolutionError::CountOverflow)?;
        }
        selected_counts.insert(locus_id.clone(), counts);
    }

    Ok(selected_counts)
}

/// Sample a marginal allele-copy population without replacement at each locus.
///
/// The caller owns stochastic coordinates and supplies a full-width priority for
/// each conceptual source allele copy. This helper owns only the exact
/// without-replacement selection/canonicalization mechanics.
pub(crate) fn sample_marginal_without_replacement<F>(
    schema: &HereditarySchema,
    source: &PopulationGeneticState,
    destination_population_id: PopulationId,
    target_census: u64,
    priority_for: F,
) -> Result<PopulationGeneticState, EvolutionError>
where
    F: FnMut(&LocusId, &AlleleId, u64) -> [u8; 32],
{
    source.validate(schema)?;
    if target_census == 0 {
        return Err(EvolutionError::DemographicCensusMustBePositive);
    }
    if target_census > source.census_individuals {
        return Err(EvolutionError::DemographicSampleExceedsSourceCensus {
            source_census: source.census_individuals,
            target_census,
        });
    }

    let target_copies = target_census
        .checked_mul(u64::from(schema.ploidy))
        .ok_or(EvolutionError::CountOverflow)?;
    let destination_counts = sample_marginal_copy_counts_without_replacement(
        schema,
        source,
        target_copies,
        priority_for,
    )?;

    PopulationGeneticState::from_counts(
        destination_population_id,
        schema,
        target_census,
        destination_counts,
    )
}
