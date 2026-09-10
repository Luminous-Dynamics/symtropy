// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic heredity and population-genetics primitives for Symtropy.
//!
//! This crate owns hereditary/population semantics only. Ecology, fitness truth,
//! morphology, rendering, persistent organism identity, speciation, cognition,
//! and civilization remain external authorities.

mod canonical;
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

pub use error::EvolutionError;
pub use heredity::{HereditaryState, HereditaryStateDigest};
pub use ids::{
    AlleleId, EvolutionExperimentId, HereditarySchemaId, LocusId, OperatorProfileId,
    PopulationId, PopulationProcessProfileId, PopulationStructureProfileId,
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

pub const EVOLUTION_SCHEMA_VERSION: u32 = 1;
pub const PROBABILITY_SCALE_PPM: u32 = 1_000_000;
