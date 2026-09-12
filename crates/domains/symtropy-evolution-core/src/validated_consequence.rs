use crate::{
    ChromosomeMap, ConsequenceError, EvolutionaryContextRef, EvolutionaryContextRefDigest,
    ExplicitConsequenceLedger, ExplicitConsequenceLedgerDigest, ExplicitLinkedPopulationCensus,
    ExplicitLinkedPopulationCensusDigest, HereditarySchema, LinkedIndividualSubject, PopulationId,
};

/// Point-in-time capability proving that one consequence ledger was replayed
/// against the exact current authorities supplied by the caller.
///
/// This type is deliberately not serializable. A persisted ledger or raw digest
/// must regain authority by replaying validation and constructing a fresh
/// capability rather than restoring one from bytes.
#[derive(Debug)]
#[must_use = "validated consequence authority should be consumed by downstream evidence APIs"]
pub struct ValidatedConsequenceLedger<'a> {
    ledger: &'a ExplicitConsequenceLedger,
    ledger_digest: ExplicitConsequenceLedgerDigest,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
}

impl<'a> ValidatedConsequenceLedger<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        ledger: &'a ExplicitConsequenceLedger,
        expected_population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
        context: &EvolutionaryContextRef,
    ) -> Result<Self, ConsequenceError> {
        ledger.validate_current(
            expected_population_id,
            schema,
            chromosome_map,
            census,
            subjects,
            context,
        )?;

        Ok(Self {
            ledger,
            ledger_digest: ledger.canonical_digest()?,
            population_id: expected_population_id.clone(),
            census_digest: census.canonical_digest()?,
            context_digest: context.canonical_digest()?,
        })
    }

    pub fn ledger(&self) -> &'a ExplicitConsequenceLedger {
        self.ledger
    }

    pub fn ledger_digest(&self) -> ExplicitConsequenceLedgerDigest {
        self.ledger_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn census_digest(&self) -> ExplicitLinkedPopulationCensusDigest {
        self.census_digest
    }

    pub fn context_digest(&self) -> EvolutionaryContextRefDigest {
        self.context_digest
    }
}
