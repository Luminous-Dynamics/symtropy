// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic heredity and population-genetics primitives for Symtropy.
//!
//! This crate owns hereditary/population semantics only. Ecology, fitness truth,
//! morphology, rendering, speciation, cognition, and civilization remain external authorities.

mod aggregate_resolution_loss;
mod ancestry_graph;
mod ancestry_pruning;
mod ancestry_simplification;
mod ancestry_simplified_forward;
mod ancestry_state;
mod canonical;
mod chromosome_map;
mod chromosome_recombination;
mod chromosome_stochastic;
mod demographic_admixture_execution;
mod demographic_event;
mod demographic_execution;
mod demographic_extinction_execution;
mod demographic_founder_execution;
mod demographic_history;
mod demographic_proof_bundle;
mod demographic_sampling;
mod demographic_source_authority;
mod demographic_split_execution;
mod demographic_structure_transition;
mod descendant_ancestry;
mod error;
mod gamete_ancestry;
mod haldane;
mod heredity;
mod ids;
mod linked_census_projection;
mod linked_gamete;
mod linked_gamete_evidence;
mod linked_individual;
mod linked_mutation;
mod linked_offspring;
mod linked_offspring_v2;
mod marker_recombination;
mod metapopulation;
mod mutation_fate;
mod mutation_lineage;
mod mutation_origin;
mod operators;
mod origin_aware_population;
mod origin_aware_trajectory;
mod phased_heredity;
mod population;
mod population_process;
mod population_structure;
mod population_trajectory;
mod reproduction;
mod schema;
mod structured_population_process;

pub use aggregate_resolution_loss::{
    continue_projected_census_alleles_only, AggregateResolutionLossCertificate,
    AggregateResolutionLossCertificateDigest, AggregateResolutionLossError,
    AggregateResolutionLossProfile, AlleleOnlyAggregateContinuation,
    AlleleOnlyAggregateContinuationDigest, AGGREGATE_RESOLUTION_LOSS_VERSION,
};
pub use ancestry_graph::{
    append_descendant_ancestry_to_graph, AncestryGeneration, AncestryGraphAppendProvenance,
    AncestryGraphAppendProvenanceDigest, AncestryGraphAppendResult, AncestryGraphEdge,
    AncestryGraphError, AncestryGraphNode, ModeledAncestryGraph, ModeledAncestryGraphDigest,
    ANCESTRY_GRAPH_APPEND_VERSION, MODELED_ANCESTRY_GRAPH_VERSION,
};
pub use ancestry_pruning::{
    prune_ancestry_reachability, AncestryPruningError, AncestryReachabilityPruningProvenance,
    AncestryReachabilityPruningProvenanceDigest, AncestryReachabilityPruningResult,
    AncestryRetentionSet, AncestryRetentionSetDigest, ANCESTRY_REACHABILITY_PRUNING_VERSION,
    ANCESTRY_RETENTION_SET_VERSION,
};
pub use ancestry_simplification::{
    simplify_modeled_ancestry, AncestrySimplificationError, AncestrySimplificationProfile,
    AncestrySimplificationProvenance, AncestrySimplificationProvenanceDigest,
    AncestrySimplificationResult, SimplifiedAncestryEdge, SimplifiedAncestryGraph,
    SimplifiedAncestryGraphDigest, ANCESTRY_SIMPLIFICATION_DERIVATION_VERSION,
    SIMPLIFIED_ANCESTRY_GRAPH_VERSION,
};
pub use ancestry_simplified_forward::{
    append_descendant_ancestry_to_simplified_graph, resimplify_simplified_ancestry,
    SimplifiedAncestryAppendProvenance, SimplifiedAncestryAppendProvenanceDigest,
    SimplifiedAncestryAppendResult, SimplifiedAncestryForwardError,
    SimplifiedAncestryResimplificationProvenance,
    SimplifiedAncestryResimplificationProvenanceDigest,
    SimplifiedAncestryResimplificationResult, SIMPLIFIED_ANCESTRY_APPEND_VERSION,
    SIMPLIFIED_ANCESTRY_RESIMPLIFICATION_VERSION,
};
pub use ancestry_state::{
    AncestryAuthorityError, ChromosomeAncestryState, HaplotypeAncestryClass,
    PhasedAncestryState, PhasedAncestryStateDigest, PHASED_ANCESTRY_STATE_VERSION,
};
pub use chromosome_map::{
    ChromosomeDefinition, ChromosomeLocus, ChromosomeMap, ChromosomeMapDigest,
    GeneticMapPositionMicromorgans, CHROMOSOME_MAP_VERSION,
};
pub use chromosome_recombination::{
    ChromosomeRecombinationDomain, ChromosomeRecombinationModel,
    ChromosomeRecombinationProfile, ChromosomeRecombinationProfileDigest,
    GeneticMapIntervalMicromorgans, CHROMOSOME_RECOMBINATION_PROFILE_VERSION,
};
pub use demographic_admixture_execution::{
    execute_census_preserving_pulse_admixture,
    execute_census_preserving_pulse_admixture_after_proven_history,
    DemographicAdmixtureExecutionModel, DemographicAdmixtureExecutionProvenance,
    DemographicAdmixtureExecutionProvenanceDigest, DemographicAdmixtureExecutionResult,
};
pub use demographic_event::{
    DaughterPopulation, DemographicEventDeclaration, DemographicEventDeclarationDigest,
    DemographicEventKind, DemographicEventTiming,
};
pub use demographic_execution::{
    execute_census_resize_bottleneck, execute_census_resize_bottleneck_after_proven_history,
    DemographicEventExecutionModel, DemographicEventExecutionProvenance,
    DemographicEventExecutionProvenanceDigest, DemographicEventExecutionResult,
};
pub use demographic_extinction_execution::{
    execute_structural_extinction, execute_structural_extinction_after_proven_history,
    DemographicExtinctionExecutionModel, DemographicExtinctionExecutionProvenance,
    DemographicExtinctionExecutionProvenanceDigest, DemographicExtinctionExecutionResult,
};
pub use demographic_founder_execution::{
    execute_founder_or_recolonization_sample,
    execute_founder_or_recolonization_sample_after_proven_history,
    DemographicFounderExecutionModel, DemographicFounderExecutionProvenance,
    DemographicFounderExecutionProvenanceDigest, DemographicFounderExecutionResult,
};
pub use demographic_history::{
    DemographicInterventionCursor, DemographicInterventionCursorDigest,
};
pub use demographic_proof_bundle::{
    DemographicExecutionEvidence, DemographicInterventionPrefixDigest,
    DemographicInterventionProofBundle, DemographicInterventionProofBundleDigest,
    DemographicInterventionProofBundleModel, DemographicInterventionProofStep,
};
pub use demographic_source_authority::ValidatedDemographicInterventionSource;
pub use demographic_split_execution::{
    execute_conservative_population_split,
    execute_conservative_population_split_after_proven_history,
    DemographicSplitExecutionModel, DemographicSplitExecutionProvenance,
    DemographicSplitExecutionProvenanceDigest, DemographicSplitExecutionResult,
};
pub use demographic_structure_transition::{
    DemographicStructureTransition, DemographicStructureTransitionDigest,
};
pub use descendant_ancestry::{
    derive_descendant_ancestry, DescendantAncestryDerivation,
    DescendantAncestryDerivationProvenance, DescendantAncestryDerivationProvenanceDigest,
    DescendantAncestryError, DescendantAncestryMaterialization,
    DescendantAncestryMaterializationDigest, DescendantChromosomeCopy,
    ModeledAncestryInheritanceEdge, DESCENDANT_ANCESTRY_DERIVATION_VERSION,
    DESCENDANT_ANCESTRY_MATERIALIZATION_VERSION,
};
pub use error::EvolutionError;
pub use gamete_ancestry::{
    derive_modeled_gamete_ancestry, ChromosomeGameteAncestry, GameteAncestryDerivation,
    GameteAncestryDerivationProvenance, GameteAncestryDerivationProvenanceDigest,
    GameteAncestryError, ModeledGameteAncestry, ModeledGameteAncestryDigest,
    ModeledLocusAncestry, GAMETE_ANCESTRY_DERIVATION_VERSION,
    MODELED_GAMETE_ANCESTRY_VERSION,
};
pub use haldane::{
    haldane_odd_parity_probability_ppm, HALDANE_MAX_ODD_PARITY_PPM,
    HALDANE_PARITY_ORACLE_VERSION, HALDANE_SATURATION_DISTANCE_MICROMORGANS,
};
pub use heredity::{HereditaryState, HereditaryStateDigest};
pub use ids::{
    AlleleId, AncestryCopyId, ChromosomeId, ChromosomeMapId,
    ChromosomeRecombinationProfileId, DemographicEventId, EvolutionExperimentId,
    EvolutionIndividualId, HereditarySchemaId, LocusId, OperatorProfileId, PopulationId,
    PopulationProcessProfileId, PopulationStructureProfileId, PopulationTransitionId,
    ReproductionEventId,
};
pub use linked_census_projection::{
    project_declared_linked_census, LinkedCensusProjection, LinkedCensusProjectionDigest,
    LinkedCensusProjectionError, LINKED_CENSUS_PROJECTION_VERSION,
};
pub use linked_gamete::{
    derive_zero_crossover_linked_gamete, ChromosomeHaplotypeDigest, LinkedGamete,
    LinkedGameteDerivation, LinkedGameteDerivationProvenance,
    LinkedGameteDerivationProvenanceDigest, LinkedGameteDigest,
    WholeChromosomeInheritanceSegment, LINKED_GAMETE_DERIVATION_VERSION, LINKED_GAMETE_VERSION,
};
pub use linked_gamete_evidence::{
    LinkedGameteDerivationEvidence, LinkedGameteDerivationEvidenceDigest,
    LINKED_GAMETE_DERIVATION_EVIDENCE_VERSION,
};
pub use linked_individual::{
    ExplicitLinkedPopulationCensus, ExplicitLinkedPopulationCensusDigest,
    ExplicitLinkedPopulationMember, LinkedIndividualError, LinkedIndividualManifest,
    LinkedIndividualManifestDigest, LinkedIndividualSubject,
    EXPLICIT_LINKED_POPULATION_CENSUS_VERSION, LINKED_INDIVIDUAL_MANIFEST_VERSION,
};
pub use linked_mutation::{
    execute_linked_mutations, LinkedMutationError, LinkedMutationExecution,
    LinkedMutationExecutionDigest, LinkedMutationOpportunity, LinkedMutationOpportunityDigest,
    LinkedMutationOutcome, NoMutationReason, LINKED_MUTATION_EXECUTION_VERSION,
};
pub use linked_offspring::{
    assemble_diploid_linked_offspring, DiploidLinkedOffspringDerivation,
    DiploidLinkedOffspringProvenance, DiploidLinkedOffspringProvenanceDigest,
    LinkedGameteContributionEvidence, DIPLOID_LINKED_OFFSPRING_DERIVATION_VERSION,
};
pub use linked_offspring_v2::{
    assemble_diploid_linked_offspring_from_evidence, DiploidLinkedOffspringDerivationV2,
    DiploidLinkedOffspringProvenanceV2, DiploidLinkedOffspringProvenanceV2Digest,
    LinkedGameteContributionEvidenceV2, DIPLOID_LINKED_OFFSPRING_DERIVATION_V2_VERSION,
};
pub use marker_recombination::{
    derive_marker_marginal_poisson_linked_gamete, AdjacentMarkerParityEvidence,
    CrossoverParity, MarkerMarginalChromosomeEvidence, MarkerMarginalGameteDerivation,
    MarkerMarginalGameteDerivationProvenance, MarkerMarginalGameteDerivationProvenanceDigest,
    MARKER_MARGINAL_GAMETE_DERIVATION_VERSION,
};
pub use metapopulation::{
    MetapopulationSnapshot, MetapopulationSnapshotDigest, MetapopulationSnapshotEntry,
};
pub use mutation_fate::{
    observe_mutation_fates, MutationFateAlleleCount, MutationFateAlleleOriginCount,
    MutationFateError, MutationFateLocusObservation, MutationFateObservation,
    MutationFateObservationDigest, MutationFateOriginCount, MutationFateSourceMultiplicity,
    MutationFateSubject, MUTATION_FATE_OBSERVATION_VERSION,
};
pub use mutation_lineage::{
    derive_descendant_mutation_lineage, initialize_root_mutation_lineage,
    MutationLineageEntry, MutationLineageError, MutationLineageEvent, MutationLineageState,
    MutationLineageStateDigest, MutationLineageTransitionProvenance,
    MutationLineageTransitionProvenanceDigest, MutationLineageTransitionResult,
    MUTATION_LINEAGE_STATE_VERSION, MUTATION_LINEAGE_TRANSITION_VERSION,
};
pub use mutation_origin::{
    declare_at_birth_mutation_origin, MutationOrigin, MutationOriginDigest, MutationOriginError,
    MutationOriginTiming, MUTATION_ORIGIN_VERSION,
};
pub use operators::{
    EvolutionOperatorProfile, EvolutionOperatorProfileDigest, MutationProfile,
    RecombinationMode, RecombinationProfile,
};
pub use origin_aware_population::{
    initialize_origin_aware_population_from_projection, neutral_origin_aware_population_step,
    AggregateActiveOriginCount, AggregateAlleleProvenance, AggregateLocusProvenance,
    OriginAwarePopulationError, OriginAwarePopulationState, OriginAwarePopulationStateDigest,
    OriginAwarePopulationTransitionProvenance,
    OriginAwarePopulationTransitionProvenanceDigest, OriginAwarePopulationTransitionResult,
    ORIGIN_AWARE_POPULATION_STATE_VERSION, ORIGIN_AWARE_POPULATION_TRANSITION_VERSION,
};
pub use origin_aware_trajectory::{
    continue_origin_aware_trajectory, derive_origin_fate_delta, ModeledBaselineFateDelta,
    MutationOriginFateDelta, OriginAwarePopulationTrajectoryPoint,
    OriginAwarePopulationTrajectoryPointDigest, OriginAwareTrajectoryError,
    OriginAwareTrajectoryTransition, OriginAwareTrajectoryTransitionDigest, OriginFateDelta,
    OriginFateDeltaDigest, ORIGIN_AWARE_TRAJECTORY_POINT_VERSION,
    ORIGIN_AWARE_TRAJECTORY_TRANSITION_VERSION, ORIGIN_FATE_DELTA_VERSION,
};
pub use phased_heredity::{
    ChromosomeHaplotype, PhasedChromosomeState, PhasedHereditaryState,
    PhasedHereditaryStateDigest, PHASED_HEREDITARY_STATE_VERSION,
};
pub use population::{PopulationGeneticState, PopulationGeneticStateDigest};
pub use population_process::{
    neutral_wright_fisher_step, PopulationProcessModel, PopulationProcessProfile,
    PopulationProcessProfileDigest, PopulationTransitionProvenance,
    PopulationTransitionProvenanceDigest, PopulationTransitionResult,
};
pub use population_structure::{
    ParentalSourceEdge, PopulationStructureModel, PopulationStructureProfile,
    PopulationStructureProfileDigest,
};
pub use population_trajectory::{
    PopulationGeneration, PopulationTrajectoryPoint, PopulationTrajectoryPointDigest,
};
pub use reproduction::{
    derive_offspring, OffspringDerivation, ParentHereditaryRef, ParentRole,
    ReproductionMode, ReproductionProvenance, ReproductionProvenanceDigest,
};
pub use schema::{HereditarySchema, HereditarySchemaDigest, LocusDefinition};
pub use structured_population_process::{
    structured_wright_fisher_step, StructuredPopulationTransitionProvenance,
    StructuredPopulationTransitionProvenanceDigest, StructuredPopulationTransitionResult,
};

pub const EVOLUTION_SCHEMA_VERSION: u32 = 1;
pub const PROBABILITY_SCALE_PPM: u32 = 1_000_000;
