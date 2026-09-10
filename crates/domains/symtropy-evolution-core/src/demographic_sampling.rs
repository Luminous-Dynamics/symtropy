use crate::{
    AlleleId, EvolutionError, HereditarySchema, LocusId, PopulationGeneticState, PopulationId,
};
use std::collections::BTreeMap;

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
    mut priority_for: F,
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
    let target_len = usize::try_from(target_copies).map_err(|_| EvolutionError::CountOverflow)?;
    let mut destination_counts = BTreeMap::new();

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
        destination_counts.insert(locus_id.clone(), counts);
    }

    PopulationGeneticState::from_counts(
        destination_population_id,
        schema,
        target_census,
        destination_counts,
    )
}
