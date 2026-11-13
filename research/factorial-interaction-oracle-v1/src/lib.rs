// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Cluster-aware analysis contracts for balanced 2x2x2 intervention designs.
//!
//! This crate computes prespecified factorial contrasts from repeated trials.
//! It treats sessions as nested within animals and weights animals equally,
//! preventing a heavily sampled animal from becoming hundreds of independent
//! biological replicates.
//!
//! It does not establish that the treatment assignments were randomized,
//! validate experimental ethics, or prove causal identifiability. Those claims
//! depend on the external protocol, provenance and study design.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const MAX_OBSERVATIONS: usize = 1_000_000;
const MAX_ANIMALS: usize = 10_000;
const MAX_SESSIONS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactorialCell {
    /// false = reference condition; true = intervention condition.
    pub a: bool,
    pub b: bool,
    pub c: bool,
}

impl FactorialCell {
    pub const fn all() -> [Self; 8] {
        [
            Self {
                a: false,
                b: false,
                c: false,
            },
            Self {
                a: false,
                b: false,
                c: true,
            },
            Self {
                a: false,
                b: true,
                c: false,
            },
            Self {
                a: false,
                b: true,
                c: true,
            },
            Self {
                a: true,
                b: false,
                c: false,
            },
            Self {
                a: true,
                b: false,
                c: true,
            },
            Self {
                a: true,
                b: true,
                c: false,
            },
            Self {
                a: true,
                b: true,
                c: true,
            },
        ]
    }

    fn signs(self) -> [f64; 3] {
        [
            if self.a { 1.0 } else { -1.0 },
            if self.b { 1.0 } else { -1.0 },
            if self.c { 1.0 } else { -1.0 },
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorSpec {
    pub id: String,
    pub reference_label: String,
    pub intervention_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeSpec {
    pub id: String,
    pub unit: String,
    /// Declared scale/transformation identity, e.g. raw-linear-v1.
    /// The oracle does not transform the response.
    pub scale_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FactorialProtocol {
    pub protocol_id: String,
    pub protocol_sha256: String,
    pub randomization_plan_sha256: String,
    pub analysis_plan_sha256: String,
    pub factors: [FactorSpec; 3],
    pub outcome: OutcomeSpec,
    pub minimum_replicates_per_cell: usize,
    pub minimum_animals: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FactorialObservation {
    pub animal_id: String,
    pub session_id: String,
    /// Must be unique within an animal/session regardless of condition.
    pub trial_id: String,
    /// One-based randomized/counterbalanced execution order within the session.
    pub trial_order: u64,
    pub cell: FactorialCell,
    pub response: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactorialError {
    InvalidIdentifier(&'static str),
    InvalidDigest(&'static str),
    InvalidFactorSpecification,
    InvalidOutcomeSpecification,
    InvalidMinimumReplicates,
    InvalidMinimumAnimals,
    EmptyObservations,
    ResourceLimitExceeded,
    NonFiniteResponse,
    NonFiniteSummary,
    ZeroTrialOrder,
    DuplicateTrialId,
    DuplicateTrialOrder,
    NonContiguousTrialOrder {
        animal_id: String,
        session_id: String,
    },
    MissingTrialOrderData,
    TooFewAnimals {
        observed: usize,
        minimum: usize,
    },
    TooFewReplicates {
        animal_id: String,
        session_id: String,
        cell: FactorialCell,
        observed: usize,
        minimum: usize,
    },
    MissingFactorialCell {
        animal_id: String,
        session_id: String,
        cell: FactorialCell,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FactorialContrasts {
    /// Main effects are high-minus-low contrasts averaged over the other factors.
    pub a: f64,
    pub b: f64,
    pub c: f64,
    /// Difference-in-differences contrasts for two-way interactions.
    pub ab: f64,
    pub ac: f64,
    pub bc: f64,
    /// Three-way difference-in-differences contrast.
    pub abc: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellOrderSummary {
    pub cell: FactorialCell,
    pub trial_count: usize,
    pub first_order: u64,
    pub last_order: u64,
    pub mean_order: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SessionContrast {
    pub animal_id: String,
    pub session_id: String,
    pub trial_count: usize,
    /// Execution-order diagnostics by factorial cell; not an order adjustment.
    pub order_by_cell: Vec<CellOrderSummary>,
    pub contrasts: FactorialContrasts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimalContrast {
    pub animal_id: String,
    pub session_count: usize,
    pub contrasts: FactorialContrasts,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContrastEstimate {
    pub mean: f64,
    /// Sample standard deviation across animal-level estimates; absent for n < 2.
    pub standard_deviation: Option<f64>,
    /// Standard error across animal-level estimates; absent for n < 2.
    pub standard_error: Option<f64>,
    pub animal_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopulationContrasts {
    pub a: ContrastEstimate,
    pub b: ContrastEstimate,
    pub c: ContrastEstimate,
    pub ab: ContrastEstimate,
    pub ac: ContrastEstimate,
    pub bc: ContrastEstimate,
    pub abc: ContrastEstimate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FactorialResult {
    pub protocol_sha256: String,
    pub randomization_plan_sha256: String,
    pub analysis_plan_sha256: String,
    pub outcome_id: String,
    pub outcome_unit: String,
    pub outcome_scale_id: String,
    pub animal_count: usize,
    pub session_count: usize,
    pub trial_count: usize,
    /// Sorted by animal_id then session_id.
    pub per_session: Vec<SessionContrast>,
    /// Sorted by animal_id; each animal contributes exactly once to population estimates.
    pub per_animal: Vec<AnimalContrast>,
    pub population: PopulationContrasts,
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_protocol(protocol: &FactorialProtocol) -> Result<(), FactorialError> {
    if !valid_identifier(&protocol.protocol_id) {
        return Err(FactorialError::InvalidIdentifier("protocol_id"));
    }
    if !valid_sha256(&protocol.protocol_sha256) {
        return Err(FactorialError::InvalidDigest("protocol"));
    }
    if !valid_sha256(&protocol.randomization_plan_sha256) {
        return Err(FactorialError::InvalidDigest("randomization_plan"));
    }
    if !valid_sha256(&protocol.analysis_plan_sha256) {
        return Err(FactorialError::InvalidDigest("analysis_plan"));
    }
    if protocol.minimum_replicates_per_cell == 0 {
        return Err(FactorialError::InvalidMinimumReplicates);
    }
    if protocol.minimum_animals == 0 {
        return Err(FactorialError::InvalidMinimumAnimals);
    }

    let mut factor_ids = BTreeSet::new();
    for factor in &protocol.factors {
        if !valid_identifier(&factor.id)
            || !valid_identifier(&factor.reference_label)
            || !valid_identifier(&factor.intervention_label)
            || factor.reference_label == factor.intervention_label
            || !factor_ids.insert(factor.id.as_str())
        {
            return Err(FactorialError::InvalidFactorSpecification);
        }
    }

    if !valid_identifier(&protocol.outcome.id)
        || !valid_identifier(&protocol.outcome.unit)
        || !valid_identifier(&protocol.outcome.scale_id)
    {
        return Err(FactorialError::InvalidOutcomeSpecification);
    }

    Ok(())
}

fn multiply_signs(signs: &[f64; 3], factors: &[usize]) -> f64 {
    factors.iter().map(|index| signs[*index]).product()
}

fn calculate_session_contrasts(
    means: &BTreeMap<FactorialCell, f64>,
) -> Result<FactorialContrasts, FactorialError> {
    let combinations: [&[usize]; 7] = [&[0], &[1], &[2], &[0, 1], &[0, 2], &[1, 2], &[0, 1, 2]];
    let mut positive: [Vec<f64>; 7] = std::array::from_fn(|_| Vec::with_capacity(4));
    let mut negative: [Vec<f64>; 7] = std::array::from_fn(|_| Vec::with_capacity(4));

    for cell in FactorialCell::all() {
        let response = means[&cell];
        let signs = cell.signs();
        for (index, factors) in combinations.iter().enumerate() {
            if multiply_signs(&signs, factors) > 0.0 {
                positive[index].push(response);
            } else {
                negative[index].push(response);
            }
        }
    }

    // Each conventional factorial effect is the difference between the
    // average cell means with positive and negative contrast signs.
    let mut effects = [0.0_f64; 7];
    for index in 0..7 {
        effects[index] = mean(&positive[index])? - mean(&negative[index])?;
        if !effects[index].is_finite() {
            return Err(FactorialError::NonFiniteSummary);
        }
    }

    Ok(FactorialContrasts {
        a: effects[0],
        b: effects[1],
        c: effects[2],
        ab: effects[3],
        ac: effects[4],
        bc: effects[5],
        abc: effects[6],
    })
}
/// Computes a finite mean without overflowing on an intermediate sum.
///
/// Values are first scaled by their largest absolute magnitude. A compensated
/// sum is then accumulated in the bounded [-1, 1] domain and rescaled once.
fn mean(values: &[f64]) -> Result<f64, FactorialError> {
    if values.is_empty() {
        return Err(FactorialError::EmptyObservations);
    }

    let mut scale = 0.0_f64;
    for value in values {
        if !value.is_finite() {
            return Err(FactorialError::NonFiniteSummary);
        }
        scale = scale.max(value.abs());
    }
    if scale == 0.0 {
        return Ok(0.0);
    }

    let mut sum = 0.0_f64;
    let mut correction = 0.0_f64;
    for value in values {
        let adjusted = value / scale - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }

    let scaled_mean = sum / values.len() as f64;
    let result = scaled_mean * scale;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(FactorialError::NonFiniteSummary)
    }
}

fn mean_contrasts(values: &[FactorialContrasts]) -> Result<FactorialContrasts, FactorialError> {
    Ok(FactorialContrasts {
        a: mean(&values.iter().map(|value| value.a).collect::<Vec<_>>())?,
        b: mean(&values.iter().map(|value| value.b).collect::<Vec<_>>())?,
        c: mean(&values.iter().map(|value| value.c).collect::<Vec<_>>())?,
        ab: mean(&values.iter().map(|value| value.ab).collect::<Vec<_>>())?,
        ac: mean(&values.iter().map(|value| value.ac).collect::<Vec<_>>())?,
        bc: mean(&values.iter().map(|value| value.bc).collect::<Vec<_>>())?,
        abc: mean(&values.iter().map(|value| value.abc).collect::<Vec<_>>())?,
    })
}
fn estimate(values: &[f64]) -> Result<ContrastEstimate, FactorialError> {
    let average = mean(values)?;
    let standard_deviation = if values.len() < 2 {
        None
    } else {
        let mut squared_deviations = 0.0_f64;
        for value in values {
            let delta = value - average;
            let squared = delta * delta;
            squared_deviations += squared;
            if !delta.is_finite() || !squared.is_finite() || !squared_deviations.is_finite() {
                return Err(FactorialError::NonFiniteSummary);
            }
        }
        Some((squared_deviations / (values.len() - 1) as f64).sqrt())
    };
    let standard_error = standard_deviation.map(|value| value / (values.len() as f64).sqrt());

    if standard_deviation.is_some_and(|value| !value.is_finite())
        || standard_error.is_some_and(|value| !value.is_finite())
    {
        return Err(FactorialError::NonFiniteSummary);
    }

    Ok(ContrastEstimate {
        mean: average,
        standard_deviation,
        standard_error,
        animal_count: values.len(),
    })
}
fn population_estimates(values: &[AnimalContrast]) -> Result<PopulationContrasts, FactorialError> {
    let a = values
        .iter()
        .map(|item| item.contrasts.a)
        .collect::<Vec<_>>();
    let b = values
        .iter()
        .map(|item| item.contrasts.b)
        .collect::<Vec<_>>();
    let c = values
        .iter()
        .map(|item| item.contrasts.c)
        .collect::<Vec<_>>();
    let ab = values
        .iter()
        .map(|item| item.contrasts.ab)
        .collect::<Vec<_>>();
    let ac = values
        .iter()
        .map(|item| item.contrasts.ac)
        .collect::<Vec<_>>();
    let bc = values
        .iter()
        .map(|item| item.contrasts.bc)
        .collect::<Vec<_>>();
    let abc = values
        .iter()
        .map(|item| item.contrasts.abc)
        .collect::<Vec<_>>();

    Ok(PopulationContrasts {
        a: estimate(&a)?,
        b: estimate(&b)?,
        c: estimate(&c)?,
        ab: estimate(&ab)?,
        ac: estimate(&ac)?,
        bc: estimate(&bc)?,
        abc: estimate(&abc)?,
    })
}
/// Compute balanced factorial contrasts while preserving animal-level clustering.
///
/// Each animal/session must contain all eight cells with at least the protocol's
/// minimum number of trials in every cell. Trials are averaged within a cell,
/// contrasts are calculated per session, sessions are averaged within animal,
/// and animals are then weighted equally in the population estimate.
///
/// This is a contrast estimator, not a full hierarchical mixed-effects model,
/// randomization test or causal-identifiability proof.
pub fn analyze_factorial(
    protocol: &FactorialProtocol,
    observations: &[FactorialObservation],
) -> Result<FactorialResult, FactorialError> {
    validate_protocol(protocol)?;

    if observations.is_empty() {
        return Err(FactorialError::EmptyObservations);
    }
    if observations.len() > MAX_OBSERVATIONS {
        return Err(FactorialError::ResourceLimitExceeded);
    }

    let mut trial_ids = BTreeSet::new();
    let mut trial_orders: BTreeMap<(String, String), BTreeSet<u64>> = BTreeMap::new();
    let mut cell_orders: BTreeMap<(String, String, FactorialCell), Vec<u64>> = BTreeMap::new();
    let mut sessions: BTreeMap<(String, String), BTreeMap<FactorialCell, Vec<f64>>> =
        BTreeMap::new();

    for observation in observations {
        if !valid_identifier(&observation.animal_id) {
            return Err(FactorialError::InvalidIdentifier("animal_id"));
        }
        if !valid_identifier(&observation.session_id) {
            return Err(FactorialError::InvalidIdentifier("session_id"));
        }
        if !valid_identifier(&observation.trial_id) {
            return Err(FactorialError::InvalidIdentifier("trial_id"));
        }
        if !observation.response.is_finite() {
            return Err(FactorialError::NonFiniteResponse);
        }
        if observation.trial_order == 0 {
            return Err(FactorialError::ZeroTrialOrder);
        }

        if !trial_orders
            .entry((
                observation.animal_id.clone(),
                observation.session_id.clone(),
            ))
            .or_default()
            .insert(observation.trial_order)
        {
            return Err(FactorialError::DuplicateTrialOrder);
        }

        cell_orders
            .entry((
                observation.animal_id.clone(),
                observation.session_id.clone(),
                observation.cell,
            ))
            .or_default()
            .push(observation.trial_order);

        let trial_key = (
            observation.animal_id.clone(),
            observation.session_id.clone(),
            observation.trial_id.clone(),
        );
        if !trial_ids.insert(trial_key) {
            return Err(FactorialError::DuplicateTrialId);
        }

        sessions
            .entry((
                observation.animal_id.clone(),
                observation.session_id.clone(),
            ))
            .or_default()
            .entry(observation.cell)
            .or_default()
            .push(observation.response);
    }

    if sessions.len() > MAX_SESSIONS {
        return Err(FactorialError::ResourceLimitExceeded);
    }

    // A one-based contiguous sequence detects duplicate/omitted event numbers.
    // It does not prove that the protocol actually randomized or counterbalanced
    // the treatment order; the summaries below keep that design auditable.
    for ((animal_id, session_id), orders) in &trial_orders {
        let expected_last =
            u64::try_from(orders.len()).map_err(|_| FactorialError::ResourceLimitExceeded)?;
        if orders.first() != Some(&1) || orders.last() != Some(&expected_last) {
            return Err(FactorialError::NonContiguousTrialOrder {
                animal_id: animal_id.clone(),
                session_id: session_id.clone(),
            });
        }
    }

    let animal_ids = sessions
        .keys()
        .map(|(animal_id, _)| animal_id.clone())
        .collect::<BTreeSet<_>>();
    if animal_ids.len() > MAX_ANIMALS {
        return Err(FactorialError::ResourceLimitExceeded);
    }
    if animal_ids.len() < protocol.minimum_animals {
        return Err(FactorialError::TooFewAnimals {
            observed: animal_ids.len(),
            minimum: protocol.minimum_animals,
        });
    }

    let mut per_session = Vec::with_capacity(sessions.len());
    let mut contrasts_by_animal: BTreeMap<String, Vec<FactorialContrasts>> = BTreeMap::new();

    for ((animal_id, session_id), cell_trials) in sessions {
        let mut cell_means = BTreeMap::new();
        let mut session_trial_count = 0;
        let mut order_by_cell = Vec::with_capacity(8);

        for cell in FactorialCell::all() {
            let Some(values) = cell_trials.get(&cell) else {
                return Err(FactorialError::MissingFactorialCell {
                    animal_id,
                    session_id,
                    cell,
                });
            };

            if values.len() < protocol.minimum_replicates_per_cell {
                return Err(FactorialError::TooFewReplicates {
                    animal_id,
                    session_id,
                    cell,
                    observed: values.len(),
                    minimum: protocol.minimum_replicates_per_cell,
                });
            }

            let orders = cell_orders
                .get(&(animal_id.clone(), session_id.clone(), cell))
                .ok_or(FactorialError::MissingTrialOrderData)?;
            if orders.len() != values.len() {
                return Err(FactorialError::MissingTrialOrderData);
            }
            let first_order = orders
                .iter()
                .copied()
                .min()
                .ok_or(FactorialError::MissingTrialOrderData)?;
            let last_order = orders
                .iter()
                .copied()
                .max()
                .ok_or(FactorialError::MissingTrialOrderData)?;
            let mean_order = mean(&orders.iter().map(|order| *order as f64).collect::<Vec<_>>())?;

            order_by_cell.push(CellOrderSummary {
                cell,
                trial_count: orders.len(),
                first_order,
                last_order,
                mean_order,
            });
            session_trial_count += values.len();
            cell_means.insert(cell, mean(values)?);
        }

        let contrasts = calculate_session_contrasts(&cell_means)?;
        per_session.push(SessionContrast {
            animal_id: animal_id.clone(),
            session_id,
            trial_count: session_trial_count,
            order_by_cell,
            contrasts,
        });
        contrasts_by_animal
            .entry(animal_id)
            .or_default()
            .push(contrasts);
    }

    let mut per_animal = Vec::with_capacity(contrasts_by_animal.len());
    for (animal_id, session_contrasts) in contrasts_by_animal {
        per_animal.push(AnimalContrast {
            animal_id,
            session_count: session_contrasts.len(),
            contrasts: mean_contrasts(&session_contrasts)?,
        });
    }

    let population = population_estimates(&per_animal)?;
    let trial_count = per_session.iter().map(|item| item.trial_count).sum();

    Ok(FactorialResult {
        protocol_sha256: protocol.protocol_sha256.clone(),
        randomization_plan_sha256: protocol.randomization_plan_sha256.clone(),
        analysis_plan_sha256: protocol.analysis_plan_sha256.clone(),
        outcome_id: protocol.outcome.id.clone(),
        outcome_unit: protocol.outcome.unit.clone(),
        outcome_scale_id: protocol.outcome.scale_id.clone(),
        animal_count: per_animal.len(),
        session_count: per_session.len(),
        trial_count,
        per_session,
        per_animal,
        population,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> String {
        "a".repeat(64)
    }

    fn protocol() -> FactorialProtocol {
        FactorialProtocol {
            protocol_id: "cpg-reflex-load-v1".to_string(),
            protocol_sha256: digest(),
            randomization_plan_sha256: digest(),
            analysis_plan_sha256: digest(),
            factors: [
                FactorSpec {
                    id: "central-drive".to_string(),
                    reference_label: "drive-off".to_string(),
                    intervention_label: "drive-on".to_string(),
                },
                FactorSpec {
                    id: "limit-reflex".to_string(),
                    reference_label: "reflex-off".to_string(),
                    intervention_label: "reflex-on".to_string(),
                },
                FactorSpec {
                    id: "load-feedback".to_string(),
                    reference_label: "load-off".to_string(),
                    intervention_label: "load-on".to_string(),
                },
            ],
            outcome: OutcomeSpec {
                id: "tripod-coordination".to_string(),
                unit: "normalized-score".to_string(),
                scale_id: "raw-linear-v1".to_string(),
            },
            minimum_replicates_per_cell: 2,
            minimum_animals: 1,
        }
    }

    fn response_with_effects(cell: FactorialCell, intercept: f64, coefficients: [f64; 7]) -> f64 {
        let [a, b, c] = cell.signs();
        intercept
            + coefficients[0] * a
            + coefficients[1] * b
            + coefficients[2] * c
            + coefficients[3] * a * b
            + coefficients[4] * a * c
            + coefficients[5] * b * c
            + coefficients[6] * a * b * c
    }

    fn observations_for_session(
        animal_id: &str,
        session_id: &str,
        intercept: f64,
        coefficients: [f64; 7],
        replicates_per_cell: usize,
    ) -> Vec<FactorialObservation> {
        let mut observations = Vec::new();
        for cell in FactorialCell::all() {
            for replicate in 0..replicates_per_cell {
                observations.push(FactorialObservation {
                    animal_id: animal_id.to_string(),
                    session_id: session_id.to_string(),
                    trial_id: format!(
                        "{}-{}-{}-{}-{}-{}",
                        animal_id, session_id, cell.a as u8, cell.b as u8, cell.c as u8, replicate
                    ),
                    trial_order: observations.len() as u64 + 1,
                    cell,
                    response: response_with_effects(cell, intercept, coefficients),
                });
            }
        }
        observations
    }

    #[test]
    fn recovers_all_main_and_interaction_effects_from_known_answer_data() {
        let mut observations = observations_for_session(
            "fly-1",
            "session-1",
            10.0,
            [1.5, -0.5, 2.0, 3.0, 0.75, -1.0, 4.0],
            2,
        );
        // Add one extra replicate to one cell: within-cell averaging must not
        // change the balanced cell-level factorial contrast.
        let cell = FactorialCell {
            a: true,
            b: true,
            c: true,
        };
        observations.push(FactorialObservation {
            animal_id: "fly-1".to_string(),
            session_id: "session-1".to_string(),
            trial_id: "fly-1-session-1-1-1-1-2".to_string(),
            trial_order: 17,
            cell,
            response: response_with_effects(cell, 10.0, [1.5, -0.5, 2.0, 3.0, 0.75, -1.0, 4.0]),
        });

        let result = analyze_factorial(&protocol(), &observations).expect("factorial analysis");
        let effects = result.population;
        assert_eq!(result.animal_count, 1);
        assert_eq!(result.session_count, 1);
        assert_eq!(result.trial_count, 17);
        assert!((effects.a.mean - 3.0).abs() < 1.0e-12);
        assert!((effects.b.mean + 1.0).abs() < 1.0e-12);
        assert!((effects.c.mean - 4.0).abs() < 1.0e-12);
        assert!((effects.ab.mean - 6.0).abs() < 1.0e-12);
        assert!((effects.ac.mean - 1.5).abs() < 1.0e-12);
        assert!((effects.bc.mean + 2.0).abs() < 1.0e-12);
        assert!((effects.abc.mean - 8.0).abs() < 1.0e-12);
        assert_eq!(effects.a.standard_error, None);
    }

    #[test]
    fn zero_interaction_negative_control_returns_zero_interactions() {
        let observations = observations_for_session(
            "fly-1",
            "session-1",
            8.0,
            [1.0, 2.0, 3.0, 0.0, 0.0, 0.0, 0.0],
            2,
        );
        let result = analyze_factorial(&protocol(), &observations).expect("factorial analysis");
        assert!(result.population.ab.mean.abs() < 1.0e-12);
        assert!(result.population.ac.mean.abs() < 1.0e-12);
        assert!(result.population.bc.mean.abs() < 1.0e-12);
        assert!(result.population.abc.mean.abs() < 1.0e-12);
    }

    #[test]
    fn animals_are_weighted_equally_despite_different_session_counts() {
        let mut observations = Vec::new();
        observations.extend(observations_for_session(
            "fly-a",
            "session-1",
            0.0,
            [2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            2,
        ));
        observations.extend(observations_for_session(
            "fly-a",
            "session-2",
            0.0,
            [2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            2,
        ));
        observations.extend(observations_for_session(
            "fly-b",
            "session-1",
            0.0,
            [4.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            2,
        ));
        let result = analyze_factorial(&protocol(), &observations).expect("factorial analysis");
        assert_eq!(result.animal_count, 2);
        assert_eq!(result.session_count, 3);
        assert!((result.population.a.mean - 6.0).abs() < 1.0e-12);
        assert!(
            (result.population.a.standard_deviation.expect("sd") - 2.8284271247461903).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn input_order_does_not_change_contrast_result() {
        let observations = observations_for_session(
            "fly-1",
            "session-1",
            5.0,
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0],
            2,
        );
        let forward = analyze_factorial(&protocol(), &observations).expect("forward");
        let mut reversed = observations.clone();
        reversed.reverse();
        let reverse = analyze_factorial(&protocol(), &reversed).expect("reverse");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn rejects_missing_factorial_cell_instead_of_imputing() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        observations.retain(|item| {
            item.cell
                != (FactorialCell {
                    a: true,
                    b: true,
                    c: true,
                })
        });
        assert!(matches!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::MissingFactorialCell { .. })
        ));
    }

    #[test]
    fn rejects_insufficient_cell_replicates() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        observations.pop();
        assert!(matches!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::TooFewReplicates { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_trial_ids_within_a_session() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        observations[1].trial_id = observations[0].trial_id.clone();
        assert_eq!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::DuplicateTrialId)
        );
    }

    #[test]
    fn reports_per_cell_trial_order_for_order_audit() {
        let observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        let result = analyze_factorial(&protocol(), &observations).expect("analysis");
        let order = &result.per_session[0].order_by_cell;
        assert_eq!(order.len(), 8);
        assert_eq!(order[0].first_order, 1);
        assert_eq!(order[0].last_order, 2);
        assert_eq!(order[0].mean_order, 1.5);
        assert_eq!(order[7].first_order, 15);
        assert_eq!(order[7].last_order, 16);
        assert_eq!(order[7].mean_order, 15.5);
    }

    #[test]
    fn rejects_duplicate_trial_order_within_a_session() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        observations[1].trial_order = observations[0].trial_order;
        assert_eq!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::DuplicateTrialOrder)
        );
    }

    #[test]
    fn rejects_gaps_in_the_trial_order_sequence() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        observations[15].trial_order = 99;
        assert!(matches!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::NonContiguousTrialOrder { .. })
        ));
    }

    #[test]
    fn rejects_zero_trial_order() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        observations[0].trial_order = 0;
        assert_eq!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::ZeroTrialOrder)
        );
    }

    #[test]
    fn rejects_non_finite_outcomes() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        observations[0].response = f64::NAN;
        assert_eq!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::NonFiniteResponse)
        );
    }

    #[test]
    fn rejects_invalid_protocol_and_randomization_digests() {
        let mut invalid = protocol();
        invalid.randomization_plan_sha256 = "not-a-digest".to_string();
        assert_eq!(
            analyze_factorial(
                &invalid,
                &observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2)
            ),
            Err(FactorialError::InvalidDigest("randomization_plan"))
        );
    }

    #[test]
    fn rejects_zero_replicate_requirement() {
        let mut invalid = protocol();
        invalid.minimum_replicates_per_cell = 0;
        assert_eq!(
            analyze_factorial(
                &invalid,
                &observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2)
            ),
            Err(FactorialError::InvalidMinimumReplicates)
        );
    }

    #[test]
    fn enforces_minimum_animal_count() {
        let mut invalid = protocol();
        invalid.minimum_animals = 2;
        let observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        assert_eq!(
            analyze_factorial(&invalid, &observations),
            Err(FactorialError::TooFewAnimals {
                observed: 1,
                minimum: 2
            })
        );
    }

    #[test]
    fn rejects_finite_inputs_that_overflow_a_factorial_contrast() {
        let mut observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        for observation in &mut observations {
            observation.response = if observation.cell.a {
                f64::MAX
            } else {
                -f64::MAX
            };
        }
        assert_eq!(
            analyze_factorial(&protocol(), &observations),
            Err(FactorialError::NonFiniteSummary)
        );
    }

    #[test]
    fn scaled_mean_avoids_overflow_for_large_same_sign_values() {
        assert_eq!(mean(&[f64::MAX, f64::MAX]), Ok(f64::MAX));
    }

    #[test]
    fn scaled_mean_preserves_finite_cancellation() {
        assert_eq!(mean(&[f64::MAX, -f64::MAX]), Ok(0.0));
    }

    #[test]
    fn scaled_mean_rejects_empty_and_non_finite_inputs() {
        assert_eq!(mean(&[]), Err(FactorialError::EmptyObservations));
        assert_eq!(
            mean(&[f64::INFINITY]),
            Err(FactorialError::NonFiniteSummary)
        );
    }

    #[test]
    fn rejects_duplicate_factor_ids() {
        let mut invalid = protocol();
        invalid.factors[1].id = invalid.factors[0].id.clone();
        let observations = observations_for_session("fly-1", "session-1", 0.0, [0.0; 7], 2);
        assert_eq!(
            analyze_factorial(&invalid, &observations),
            Err(FactorialError::InvalidFactorSpecification)
        );
    }
}
