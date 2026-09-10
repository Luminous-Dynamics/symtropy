// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic heredity and population-genetics primitives for Symtropy.
//!
//! This crate owns hereditary/population semantics only. Ecology, fitness truth,
//! morphology, rendering, persistent organism identity, speciation, cognition,
//! and civilization remain external authorities.

mod canonical;
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
mod error;
mod heredity;
mod ids;
mod metapopulation;
mod operators;
mod population;
mod population_process;
mod population_structure;
mod population_trajectory;
mod reproduction;
mod schema;
mod structured_population_process;

pub use demographic_admixture_execution::{
    execute_census_preserving_pulse_admixture, DemographicAdmixtureExecutionModel,
    DemographicAdmixtureExecutionProvenance, DemographicAdmixtureExecutionProvenanceDigest,
    DemographicAdmixtureExecutionResult,
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
    execute_structural_extinction, DemographicExtinctionExecutionModel,
    DemographicExtinctionExecutionProvenance, DemographicExtinctionExecutionProvenanceDigest,
    DemographicExtinctionExecutionResult,
};
pub use demographic_founder_execution::{
    execute_founder_or_recolonization_sample, DemographicFounderExecutionModel,
    DemographicFounderExecutionProvenance, DemographicFounderExecutionProvenanceDigest,
    DemographicFounderExecutionResult,
};
pub use demographic_history::{
    DemographicInterventionCursor, DemographicInterventionCursorDigest,
};
pub use demographic_proof_bundle::{
    DemographicExecutionEvidence, DemographicInterventionProofBundle,
    DemographicInterventionProofBundleDigest, DemographicInterventionProofBundleModel,
    DemographicInterventionProofStep,
};
pub use demographic_source_authority::ValidatedDemographicInterventionSource;
pub use demographic_split_execution::{
    execute_conservative_population_split, DemographicSplitExecutionModel,
    DemographicSplitExecutionProvenance, DemographicSplitExecutionProvenanceDigest,
    DemographicSplitExecutionResult,
};
pub use demographic_structure_transition::{
    DemographicStructureTransition, DemographicStructureTransitionDigest,
};
pub use error::EvolutionError;
pub use heredity::{HereditaryState, HereditaryStateDigest};
pub use ids::{
    AlleleId, DemographicEventId, EvolutionExperimentId, HereditarySchemaId, LocusId,
    OperatorProfileId, PopulationId, PopulationProcessProfileId, PopulationStructureProfileId,
    PopulationTransitionId, ReproductionEventId,
};
pub use metapopulation::{
    MetapopulationSnapshot, MetapopulationSnapshotDigest, MetapopulationSnapshotEntry,
};
pub use operators::{
    EvolutionOperatorProfile, EvolutionOperatorProfileDigest, MutationProfile,
    RecombinationMode, RecombinationProfile,
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
