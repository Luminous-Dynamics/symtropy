// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::BTreeMap;

use symtropy_lifesim_core::population::{
    CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand, PopulationState,
};
use symtropy_lifesim_core::projection::{
    ProjectionContext, ProjectionRevision, ProjectionScope, ProjectionSeed,
    SPARSE_FISHER_YATES_PROJECTION_V1, project_population_bounded,
};

const SCOPE: ProjectionScope = ProjectionScope(0xA11CE);

fn distribution<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
    CountDistribution::new(pairs.into_iter().collect()).unwrap()
}

fn population(biomass_milligrams: u64) -> PopulationState {
    PopulationState::new(
        10,
        biomass_milligrams,
        distribution([
            (PopulationAgeBand::Juvenile, 3),
            (PopulationAgeBand::Mature, 7),
        ]),
        distribution([
            (PopulationConditionBand::Stable, 8),
            (PopulationConditionBand::Stressed, 2),
        ]),
        distribution([
            (PopulationCell::new(0, 0, 0), 5),
            (PopulationCell::new(1, 0, 0), 5),
        ]),
    )
    .unwrap()
}

fn context(seed: u64) -> ProjectionContext {
    ProjectionContext::sparse_fisher_yates_v1(
        SCOPE,
        ProjectionRevision(7),
        ProjectionSeed(seed),
    )
}

#[test]
fn frozen_sparse_projection_vector_detects_unversioned_algorithm_drift() {
    let projection = project_population_bounded(&population(2_500_003), context(91), 5, 5).unwrap();
    let candidates = projection.candidates();

    assert_eq!(projection.context().scheme(), SPARSE_FISHER_YATES_PROJECTION_V1);
    assert_eq!(candidates.len(), 5);

    let expected = [
        (
            PopulationAgeBand::Mature,
            PopulationConditionBand::Stable,
            PopulationCell::new(0, 0, 0),
        ),
        (
            PopulationAgeBand::Mature,
            PopulationConditionBand::Stable,
            PopulationCell::new(1, 0, 0),
        ),
        (
            PopulationAgeBand::Mature,
            PopulationConditionBand::Stressed,
            PopulationCell::new(1, 0, 0),
        ),
        (
            PopulationAgeBand::Juvenile,
            PopulationConditionBand::Stable,
            PopulationCell::new(0, 0, 0),
        ),
        (
            PopulationAgeBand::Mature,
            PopulationConditionBand::Stable,
            PopulationCell::new(1, 0, 0),
        ),
    ];

    for (index, (candidate, expected)) in candidates.iter().zip(expected).enumerate() {
        assert_eq!(candidate.handle().candidate_index(), index as u64);
        assert_eq!(candidate.handle().context(), context(91));
        assert_eq!(candidate.projected_age(), expected.0);
        assert_eq!(candidate.projected_condition(), expected.1);
        assert_eq!(candidate.projected_cell(), expected.2);
    }
}

#[test]
fn projection_is_independent_of_canonical_biomass_authority() {
    let low_biomass = project_population_bounded(&population(10), context(123), 10, 10).unwrap();
    let high_biomass =
        project_population_bounded(&population(u64::MAX), context(123), 10, 10).unwrap();

    assert_eq!(low_biomass, high_biomass);
}

#[test]
fn full_level_p_projection_preserves_each_source_marginal() {
    let source = population(2_500_003);
    let projection = project_population_bounded(&source, context(44), 10, 10).unwrap();

    let mut ages = BTreeMap::new();
    let mut conditions = BTreeMap::new();
    let mut cells = BTreeMap::new();

    for candidate in projection.candidates() {
        *ages.entry(candidate.projected_age()).or_insert(0u64) += 1;
        *conditions
            .entry(candidate.projected_condition())
            .or_insert(0u64) += 1;
        *cells.entry(candidate.projected_cell()).or_insert(0u64) += 1;
    }

    assert_eq!(CountDistribution::new(ages).unwrap(), *source.age_distribution());
    assert_eq!(
        CountDistribution::new(conditions).unwrap(),
        *source.condition_distribution()
    );
    assert_eq!(
        CountDistribution::new(cells).unwrap(),
        *source.occupancy_distribution()
    );
}
