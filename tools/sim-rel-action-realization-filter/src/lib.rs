//! Exact finite-state filtering when commanded and realized interventions differ.
//!
//! The latent state is (mechanism, process, sensor, apparatus). A transition row
//! jointly emits the next latent state, realized-action class, execution readback,
//! scientific measurement and hazard flag. The kernel is not factorized.

#![forbid(unsafe_code)]

pub mod decision_value;

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_HYPOTHESES: usize = 16;
const MAX_PHYSICAL_STATES: usize = 32;
const MAX_SENSOR_STATES: usize = 32;
const MAX_APPARATUS_STATES: usize = 32;
const MAX_COMMANDS: usize = 16;
const MAX_REALIZED_ACTIONS: usize = 32;
const MAX_READBACKS: usize = 32;
const MAX_MEASUREMENTS: usize = 64;
const MAX_ROWS: usize = 1_000_000;
const MAX_OUTCOMES_PER_ROW: usize = 64;
const MAX_TOTAL_OUTCOMES: usize = 4_000_000;
const MAX_INITIAL_TUPLES: usize = 65_536;

/// A reduced non-negative rational with checked arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Rational {
    numerator: u128,
    denominator: u128,
}

impl Rational {
    pub fn new(numerator: u128, denominator: u128) -> Result<Self, String> {
        if denominator == 0 {
            return Err("rational denominator must be positive".into());
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub const fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub fn from_u64(value: u64) -> Self {
        Self {
            numerator: u128::from(value),
            denominator: 1,
        }
    }

    pub const fn numerator(self) -> u128 {
        self.numerator
    }

    pub const fn denominator(self) -> u128 {
        self.denominator
    }

    pub const fn is_zero(self) -> bool {
        self.numerator == 0
    }

    pub fn is_probability(self) -> bool {
        self <= Self::one()
    }

    pub fn checked_add(self, other: Self) -> Result<Self, String> {
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = self
            .numerator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let right = other
            .numerator
            .checked_mul(right_scale)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let numerator = left
            .checked_add(right)
            .ok_or_else(|| "rational addition numerator overflow".to_string())?;
        let denominator = self
            .denominator
            .checked_mul(left_scale)
            .ok_or_else(|| "rational addition denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, String> {
        let cross_left = gcd(self.numerator, other.denominator);
        let cross_right = gcd(other.numerator, self.denominator);
        let left_num = self.numerator / cross_left;
        let right_num = other.numerator / cross_right;
        let left_den = self.denominator / cross_right;
        let right_den = other.denominator / cross_left;
        let numerator = left_num
            .checked_mul(right_num)
            .ok_or_else(|| "rational multiplication numerator overflow".to_string())?;
        let denominator = left_den
            .checked_mul(right_den)
            .ok_or_else(|| "rational multiplication denominator overflow".to_string())?;
        Self::new(numerator, denominator)
    }

    pub fn checked_div(self, other: Self) -> Result<Self, String> {
        if other.numerator == 0 {
            return Err("rational division by zero".into());
        }
        self.checked_mul(Self::new(other.denominator, other.numerator)?)
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_fractions(
            self.numerator,
            self.denominator,
            other.numerator,
            other.denominator,
        )
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Compare non-negative fractions without cross-multiplication overflow.
fn compare_fractions(
    mut left_num: u128,
    mut left_den: u128,
    mut right_num: u128,
    mut right_den: u128,
) -> Ordering {
    let mut reversed = false;
    loop {
        let left_quotient = left_num / left_den;
        let right_quotient = right_num / right_den;
        if left_quotient != right_quotient {
            return if reversed {
                right_quotient.cmp(&left_quotient)
            } else {
                left_quotient.cmp(&right_quotient)
            };
        }

        let left_remainder = left_num % left_den;
        let right_remainder = right_num % right_den;
        match (left_remainder == 0, right_remainder == 0) {
            (true, true) => return Ordering::Equal,
            (true, false) => {
                return if reversed {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            (false, true) => {
                return if reversed {
                    Ordering::Less
                } else {
                    Ordering::Greater
                };
            }
            (false, false) => {}
        }

        left_num = left_den;
        left_den = left_remainder;
        right_num = right_den;
        right_den = right_remainder;
        reversed = !reversed;
    }
}

/// One point in the joint hidden state.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct LatentState {
    pub hypothesis: usize,
    pub physical_state: usize,
    pub sensor_state: usize,
    pub apparatus_state: usize,
}

/// One non-zero mass in a normalized joint belief.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeliefMass {
    pub state: LatentState,
    pub mass: Rational,
}

/// A possible joint result of one commanded experiment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransitionOutcome {
    pub probability: Rational,
    pub next_state: LatentState,
    pub realized_action: usize,
    pub readback: Option<usize>,
    pub measurement: Option<usize>,
    pub hazard: bool,
}

/// One row for a complete (latent state, command) pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelRow {
    pub state: LatentState,
    pub command: usize,
    pub outcomes: Vec<TransitionOutcome>,
}

/// A complete finite joint action-realization model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRealizationKernel {
    pub hypothesis_ids: Vec<String>,
    pub physical_state_count: usize,
    pub sensor_state_count: usize,
    pub apparatus_state_count: usize,
    pub command_ids: Vec<String>,
    pub realized_action_ids: Vec<String>,
    pub readback_ids: Vec<String>,
    pub measurement_ids: Vec<String>,
    pub initial_belief: Vec<BeliefMass>,
    /// Canonical order: hypothesis, physical, sensor, apparatus, command.
    pub rows: Vec<KernelRow>,
}

impl ActionRealizationKernel {
    pub fn validate(&self) -> Result<(), String> {
        let hypotheses = self.hypothesis_ids.len();
        let commands = self.command_ids.len();

        validate_ids(&self.hypothesis_ids, "hypothesis", false)?;
        validate_ids(&self.command_ids, "command", false)?;
        validate_ids(&self.realized_action_ids, "realized-action", false)?;
        validate_ids(&self.readback_ids, "readback", true)?;
        validate_ids(&self.measurement_ids, "measurement", false)?;

        check_dimension(hypotheses, MAX_HYPOTHESES, "hypothesis")?;
        check_dimension(self.physical_state_count, MAX_PHYSICAL_STATES, "physical state")?;
        check_dimension(self.sensor_state_count, MAX_SENSOR_STATES, "sensor state")?;
        check_dimension(
            self.apparatus_state_count,
            MAX_APPARATUS_STATES,
            "apparatus state",
        )?;
        check_dimension(commands, MAX_COMMANDS, "command")?;
        check_dimension(
            self.realized_action_ids.len(),
            MAX_REALIZED_ACTIONS,
            "realized action",
        )?;
        if self.readback_ids.len() > MAX_READBACKS {
            return Err("readback vocabulary exceeds its cap".into());
        }
        check_dimension(
            self.measurement_ids.len(),
            MAX_MEASUREMENTS,
            "measurement",
        )?;

        if self.initial_belief.is_empty()
            || self.initial_belief.len() > MAX_INITIAL_TUPLES
        {
            return Err("initial belief is empty or exceeds its tuple cap".into());
        }

        let mut initial_mass = Rational::zero();
        let mut initial_keys = BTreeSet::new();
        for point in &self.initial_belief {
            validate_state(self, point.state)?;
            if point.mass.is_zero() {
                return Err("initial belief contains zero mass".into());
            }
            if !initial_keys.insert(point.state) {
                return Err("initial belief contains a duplicate joint state".into());
            }
            initial_mass = initial_mass.checked_add(point.mass)?;
        }
        if initial_mass != Rational::one() {
            return Err("initial belief must sum exactly to one".into());
        }

        let row_count = hypotheses
            .checked_mul(self.physical_state_count)
            .and_then(|n| n.checked_mul(self.sensor_state_count))
            .and_then(|n| n.checked_mul(self.apparatus_state_count))
            .and_then(|n| n.checked_mul(commands))
            .ok_or_else(|| "kernel row count overflow".to_string())?;
        if row_count == 0 || row_count > MAX_ROWS || self.rows.len() != row_count {
            return Err("kernel must contain exactly one row per latent-state/command pair".into());
        }

        let mut total_outcomes = 0_usize;
        let mut row_index = 0_usize;
        for hypothesis in 0..hypotheses {
            for physical_state in 0..self.physical_state_count {
                for sensor_state in 0..self.sensor_state_count {
                    for apparatus_state in 0..self.apparatus_state_count {
                        let state = LatentState {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            apparatus_state,
                        };
                        for command in 0..commands {
                            let row = &self.rows[row_index];
                            if row.state != state || row.command != command {
                                return Err(format!(
                                    "kernel row {row_index} violates canonical ordering"
                                ));
                            }
                            if row.outcomes.is_empty()
                                || row.outcomes.len() > MAX_OUTCOMES_PER_ROW
                            {
                                return Err(format!(
                                    "kernel row {row_index} has an invalid outcome count"
                                ));
                            }
                            total_outcomes = total_outcomes
                                .checked_add(row.outcomes.len())
                                .ok_or_else(|| "outcome count overflow".to_string())?;
                            if total_outcomes > MAX_TOTAL_OUTCOMES {
                                return Err("kernel exceeds the total outcome cap".into());
                            }

                            let mut row_mass = Rational::zero();
                            for outcome in &row.outcomes {
                                validate_state(self, outcome.next_state)?;
                                if outcome.realized_action >= self.realized_action_ids.len() {
                                    return Err(format!(
                                        "kernel row {row_index} has an invalid realized action"
                                    ));
                                }
                                if outcome
                                    .readback
                                    .is_some_and(|value| value >= self.readback_ids.len())
                                {
                                    return Err(format!(
                                        "kernel row {row_index} has an invalid readback"
                                    ));
                                }
                                if outcome
                                    .measurement
                                    .is_some_and(|value| value >= self.measurement_ids.len())
                                {
                                    return Err(format!(
                                        "kernel row {row_index} has an invalid measurement"
                                    ));
                                }
                                if outcome.hazard && outcome.measurement.is_some() {
                                    return Err(
                                        "hazard outcomes cannot contain a scientific measurement"
                                            .into(),
                                    );
                                }
                                if !outcome.hazard && outcome.measurement.is_none() {
                                    return Err(
                                        "safe outcomes must contain a scientific measurement"
                                            .into(),
                                    );
                                }
                                if outcome.probability.is_zero()
                                    || !outcome.probability.is_probability()
                                {
                                    return Err(format!(
                                        "kernel row {row_index} has an invalid probability"
                                    ));
                                }
                                row_mass = row_mass.checked_add(outcome.probability)?;
                            }
                            if row_mass != Rational::one() {
                                return Err(format!(
                                    "kernel row {row_index} probability mass must sum to one"
                                ));
                            }
                            row_index += 1;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn row_index(&self, state: LatentState, command: usize) -> usize {
        ((((state.hypothesis * self.physical_state_count + state.physical_state)
            * self.sensor_state_count
            + state.sensor_state)
            * self.apparatus_state_count
            + state.apparatus_state)
            * self.command_ids.len())
            + command
    }
}

fn check_dimension(value: usize, maximum: usize, label: &str) -> Result<(), String> {
    if value == 0 || value > maximum {
        return Err(format!("{label} count must be in 1..={maximum}"));
    }
    Ok(())
}

fn validate_ids(ids: &[String], label: &str, allow_empty: bool) -> Result<(), String> {
    if (!allow_empty && ids.is_empty()) || ids.iter().any(|id| id.trim().is_empty()) {
        return Err(format!("{label} IDs must be non-empty"));
    }
    let mut unique = BTreeSet::new();
    if ids.iter().any(|id| !unique.insert(id)) {
        return Err(format!("{label} IDs must be unique"));
    }
    Ok(())
}

fn validate_state(kernel: &ActionRealizationKernel, state: LatentState) -> Result<(), String> {
    if state.hypothesis >= kernel.hypothesis_ids.len()
        || state.physical_state >= kernel.physical_state_count
        || state.sensor_state >= kernel.sensor_state_count
        || state.apparatus_state >= kernel.apparatus_state_count
    {
        return Err("latent state references an out-of-range dimension".into());
    }
    Ok(())
}

fn validate_belief(
    kernel: &ActionRealizationKernel,
    belief: &[BeliefMass],
) -> Result<(), String> {
    if belief.is_empty() || belief.len() > MAX_INITIAL_TUPLES {
        return Err("belief is empty or exceeds its tuple cap".into());
    }
    let mut seen = BTreeSet::new();
    let mut total = Rational::zero();
    for point in belief {
        validate_state(kernel, point.state)?;
        if point.mass.is_zero() {
            return Err("belief contains zero mass".into());
        }
        if !seen.insert(point.state) {
            return Err("belief contains a duplicate joint state".into());
        }
        total = total.checked_add(point.mass)?;
    }
    if total != Rational::one() {
        return Err("belief mass must sum exactly to one".into());
    }
    Ok(())
}

/// How one field of an outcome is constrained by public history.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ObservationField {
    /// The field was not observed; do not condition on it.
    Any,
    /// The field was observed to be absent (for example, a missing readback).
    Missing,
    /// The field was observed with this vocabulary ID.
    Value(usize),
}

/// The part of an outcome recorded in public history.
///
/// Explicitly distinguishing Any from Missing prevents an absent execution
/// acknowledgement from being confused with an unobserved acknowledgement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct PublicObservation {
    pub hazard: bool,
    pub readback: ObservationField,
    pub measurement: ObservationField,
}

/// An exact posterior point retaining the correlation with the realized action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PosteriorMass {
    pub next_state: LatentState,
    pub realized_action: usize,
    pub mass: Rational,
}

/// Result of conditioning on one commanded experiment and its public outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterResult {
    pub observation_likelihood: Rational,
    pub terminal_hazard: bool,
    pub joint_posterior: Vec<PosteriorMass>,
}

impl FilterResult {
    /// Marginalize the last realized-action label for the next Markov step.
    ///
    /// Persistent consequences must already be represented in next_state.
    pub fn belief_for_next_step(&self) -> Result<Vec<BeliefMass>, FilterError> {
        if self.terminal_hazard {
            return Err(FilterError::TerminalHazard);
        }
        let mut masses = BTreeMap::<LatentState, Rational>::new();
        for point in &self.joint_posterior {
            let entry = masses.entry(point.next_state).or_insert(Rational::zero());
            *entry = entry
                .checked_add(point.mass)
                .map_err(FilterError::Arithmetic)?;
        }
        Ok(masses
            .into_iter()
            .map(|(state, mass)| BeliefMass { state, mass })
            .collect())
    }
}

/// Typed failures. A zero-likelihood observation never creates an empty posterior.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilterError {
    InvalidKernel(String),
    InvalidBelief(String),
    InvalidCommand,
    InvalidObservation,
    ZeroLikelihood,
    TerminalHazard,
    Arithmetic(String),
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKernel(message) => write!(f, "invalid kernel: {message}"),
            Self::InvalidBelief(message) => write!(f, "invalid belief: {message}"),
            Self::InvalidCommand => write!(f, "command ID is out of range"),
            Self::InvalidObservation => write!(f, "observation ID is out of range"),
            Self::ZeroLikelihood => write!(f, "observation has exact zero likelihood"),
            Self::TerminalHazard => write!(f, "a terminal hazard cannot be continued"),
            Self::Arithmetic(message) => write!(f, "exact arithmetic failed: {message}"),
        }
    }
}

impl std::error::Error for FilterError {}

/// Update a joint belief after one commanded action and one public observation.
///
/// The model must include every persistent effect of an earlier realized action
/// in its apparatus/process state. This function is exact for the supplied kernel.
pub fn filter_one_step(
    kernel: &ActionRealizationKernel,
    current_belief: &[BeliefMass],
    command: usize,
    observation: PublicObservation,
) -> Result<FilterResult, FilterError> {
    kernel.validate().map_err(FilterError::InvalidKernel)?;
    validate_belief(kernel, current_belief).map_err(FilterError::InvalidBelief)?;

    if command >= kernel.command_ids.len() {
        return Err(FilterError::InvalidCommand);
    }
    if matches!(observation.readback, ObservationField::Value(value) if value >= kernel.readback_ids.len())
        || matches!(observation.measurement, ObservationField::Value(value) if value >= kernel.measurement_ids.len())
        || (observation.hazard
            && matches!(observation.measurement, ObservationField::Value(_)))
    {
        return Err(FilterError::InvalidObservation);
    }

    let mut raw = BTreeMap::<(LatentState, usize), Rational>::new();
    let mut likelihood = Rational::zero();
    for point in current_belief {
        let row = &kernel.rows[kernel.row_index(point.state, command)];
        for outcome in &row.outcomes {
            if outcome.hazard != observation.hazard {
                continue;
            }
            let readback_matches = match observation.readback {
                ObservationField::Any => true,
                ObservationField::Missing => outcome.readback.is_none(),
                ObservationField::Value(value) => outcome.readback == Some(value),
            };
            if !readback_matches {
                continue;
            }

            let measurement_matches = match observation.measurement {
                ObservationField::Any => true,
                ObservationField::Missing => outcome.measurement.is_none(),
                ObservationField::Value(value) => outcome.measurement == Some(value),
            };
            if !measurement_matches {
                continue;
            }

            let mass = point
                .mass
                .checked_mul(outcome.probability)
                .map_err(FilterError::Arithmetic)?;
            let entry = raw
                .entry((outcome.next_state, outcome.realized_action))
                .or_insert(Rational::zero());
            *entry = entry.checked_add(mass).map_err(FilterError::Arithmetic)?;
            likelihood = likelihood
                .checked_add(mass)
                .map_err(FilterError::Arithmetic)?;
        }
    }

    if likelihood.is_zero() {
        return Err(FilterError::ZeroLikelihood);
    }

    let mut joint_posterior = Vec::with_capacity(raw.len());
    for ((next_state, realized_action), mass) in raw {
        joint_posterior.push(PosteriorMass {
            next_state,
            realized_action,
            mass: mass.checked_div(likelihood).map_err(FilterError::Arithmetic)?,
        });
    }

    Ok(FilterResult {
        observation_likelihood: likelihood,
        terminal_hazard: observation.hazard,
        joint_posterior,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xor_kernel() -> ActionRealizationKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            let outcomes = (0..2)
                .map(|realized_action| TransitionOutcome {
                    probability: Rational::new(1, 2).unwrap(),
                    next_state: state,
                    realized_action,
                    readback: Some(realized_action),
                    measurement: Some(hypothesis ^ realized_action),
                    hazard: false,
                })
                .collect();
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes,
            });
        }
        ActionRealizationKernel {
            hypothesis_ids: vec!["h0".into(), "h1".into()],
            physical_state_count: 1,
            sensor_state_count: 1,
            apparatus_state_count: 1,
            command_ids: vec!["pulse".into()],
            realized_action_ids: vec!["none".into(), "delivered".into()],
            readback_ids: vec!["none".into(), "delivered".into()],
            measurement_ids: vec!["zero".into(), "one".into()],
            initial_belief: vec![
                BeliefMass {
                    state: LatentState {
                        hypothesis: 0,
                        physical_state: 0,
                        sensor_state: 0,
                        apparatus_state: 0,
                    },
                    mass: Rational::new(1, 2).unwrap(),
                },
                BeliefMass {
                    state: LatentState {
                        hypothesis: 1,
                        physical_state: 0,
                        sensor_state: 0,
                        apparatus_state: 0,
                    },
                    mass: Rational::new(1, 2).unwrap(),
                },
            ],
            rows,
        }
    }

    fn mass_for_hypothesis(result: &FilterResult, hypothesis: usize) -> Rational {
        result
            .joint_posterior
            .iter()
            .filter(|point| point.next_state.hypothesis == hypothesis)
            .fold(Rational::zero(), |sum, point| {
                sum.checked_add(point.mass).unwrap()
            })
    }

    #[test]
    fn execution_readback_resolves_xor_confounding() {
        let kernel = xor_kernel();
        let prior = kernel.initial_belief.clone();
        let measurement_only = filter_one_step(
            &kernel,
            &prior,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Any,
                measurement: ObservationField::Value(0),
            },
        )
        .unwrap();

        assert_eq!(
            measurement_only.observation_likelihood,
            Rational::new(1, 2).unwrap()
        );
        assert_eq!(
            mass_for_hypothesis(&measurement_only, 0),
            Rational::new(1, 2).unwrap()
        );
        assert_eq!(
            mass_for_hypothesis(&measurement_only, 1),
            Rational::new(1, 2).unwrap()
        );

        let execution_verified = filter_one_step(
            &kernel,
            &prior,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Value(0),
                measurement: ObservationField::Value(0),
            },
        )
        .unwrap();

        assert_eq!(
            execution_verified.observation_likelihood,
            Rational::new(1, 4).unwrap()
        );
        assert_eq!(mass_for_hypothesis(&execution_verified, 0), Rational::one());
        assert_eq!(mass_for_hypothesis(&execution_verified, 1), Rational::zero());
    }

    fn dropped_intervention_kernel() -> ActionRealizationKernel {
        let mut rows = Vec::new();
        for physical_state in 0..2 {
            let state = LatentState {
                hypothesis: 0,
                physical_state,
                sensor_state: 0,
                apparatus_state: 0,
            };
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: physical_state,
                    readback: Some(physical_state),
                    measurement: Some(physical_state),
                    hazard: false,
                }],
            });
        }
        ActionRealizationKernel {
            hypothesis_ids: vec!["h".into()],
            physical_state_count: 2,
            sensor_state_count: 1,
            apparatus_state_count: 1,
            command_ids: vec!["apply-dose".into()],
            realized_action_ids: vec!["dropped".into(), "delivered".into()],
            readback_ids: vec!["not-delivered".into(), "delivered".into()],
            measurement_ids: vec!["cold".into(), "hot".into()],
            initial_belief: vec![BeliefMass {
                state: LatentState {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state: 0,
                },
                mass: Rational::one(),
            }],
            rows,
        }
    }

    #[test]
    fn observed_missing_readback_is_distinct_from_unobserved_readback() {
        let mut kernel = xor_kernel();
        kernel.rows[0].outcomes[0].readback = None;

        let result = filter_one_step(
            &kernel,
            &kernel.initial_belief,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Missing,
                measurement: ObservationField::Value(0),
            },
        )
        .unwrap();

        assert_eq!(result.joint_posterior.len(), 1);
        assert_eq!(mass_for_hypothesis(&result, 0), Rational::one());
        assert_eq!(mass_for_hypothesis(&result, 1), Rational::zero());

        let unconstrained = filter_one_step(
            &kernel,
            &kernel.initial_belief,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Any,
                measurement: ObservationField::Value(0),
            },
        )
        .unwrap();

        assert_eq!(mass_for_hypothesis(&unconstrained, 0), Rational::new(1, 2).unwrap());
        assert_eq!(mass_for_hypothesis(&unconstrained, 1), Rational::new(1, 2).unwrap());
    }

    #[test]
    fn dropped_intervention_is_not_assumed_to_have_changed_the_specimen() {
        let kernel = dropped_intervention_kernel();
        let result = filter_one_step(
            &kernel,
            &kernel.initial_belief,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Value(0),
                measurement: ObservationField::Value(0),
            },
        )
        .unwrap();

        assert_eq!(result.joint_posterior.len(), 1);
        assert_eq!(result.joint_posterior[0].next_state.physical_state, 0);
        assert_eq!(result.joint_posterior[0].realized_action, 0);
    }

    #[test]
    fn partial_delivery_is_distinguished_from_full_delivery() {
        let mut rows = Vec::new();
        for physical_state in 0..3 {
            let state = LatentState {
                hypothesis: 0,
                physical_state,
                sensor_state: 0,
                apparatus_state: 0,
            };
            let outcomes = if physical_state == 0 {
                vec![
                    TransitionOutcome {
                        probability: Rational::new(1, 4).unwrap(),
                        next_state: state,
                        realized_action: 0,
                        readback: Some(0),
                        measurement: Some(0),
                        hazard: false,
                    },
                    TransitionOutcome {
                        probability: Rational::new(1, 2).unwrap(),
                        next_state: LatentState {
                            physical_state: 1,
                            ..state
                        },
                        realized_action: 1,
                        readback: Some(1),
                        measurement: Some(1),
                        hazard: false,
                    },
                    TransitionOutcome {
                        probability: Rational::new(1, 4).unwrap(),
                        next_state: LatentState {
                            physical_state: 2,
                            ..state
                        },
                        realized_action: 2,
                        readback: Some(2),
                        measurement: Some(2),
                        hazard: false,
                    },
                ]
            } else {
                vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: physical_state,
                    readback: Some(physical_state),
                    measurement: Some(physical_state),
                    hazard: false,
                }]
            };
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes,
            });
        }

        let kernel = ActionRealizationKernel {
            hypothesis_ids: vec!["h".into()],
            physical_state_count: 3,
            sensor_state_count: 1,
            apparatus_state_count: 1,
            command_ids: vec!["deliver-dose".into()],
            realized_action_ids: vec!["dropped".into(), "partial".into(), "full".into()],
            readback_ids: vec!["none".into(), "partial".into(), "full".into()],
            measurement_ids: vec!["cold".into(), "warm".into(), "hot".into()],
            initial_belief: vec![BeliefMass {
                state: LatentState {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state: 0,
                },
                mass: Rational::one(),
            }],
            rows,
        };

        let result = filter_one_step(
            &kernel,
            &kernel.initial_belief,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Value(1),
                measurement: ObservationField::Value(1),
            },
        )
        .unwrap();

        assert_eq!(
            result.observation_likelihood,
            Rational::new(1, 2).unwrap()
        );
        assert_eq!(result.joint_posterior.len(), 1);
        assert_eq!(result.joint_posterior[0].next_state.physical_state, 1);
        assert_eq!(result.joint_posterior[0].realized_action, 1);
    }

    fn backlash_kernel() -> ActionRealizationKernel {
        let state_zero = LatentState {
            hypothesis: 0,
            physical_state: 0,
            sensor_state: 0,
            apparatus_state: 0,
        };
        let state_one = LatentState {
            apparatus_state: 1,
            ..state_zero
        };
        ActionRealizationKernel {
            hypothesis_ids: vec!["h".into()],
            physical_state_count: 1,
            sensor_state_count: 1,
            apparatus_state_count: 2,
            command_ids: vec!["same-command".into()],
            realized_action_ids: vec!["none".into(), "normal-pulse".into(), "backlash-pulse".into()],
            readback_ids: vec!["none".into(), "normal".into(), "backlash".into()],
            measurement_ids: vec!["zero".into(), "one".into()],
            initial_belief: vec![BeliefMass {
                state: state_zero,
                mass: Rational::one(),
            }],
            rows: vec![
                KernelRow {
                    state: state_zero,
                    command: 0,
                    outcomes: vec![TransitionOutcome {
                        probability: Rational::one(),
                        next_state: state_one,
                        realized_action: 1,
                        readback: Some(1),
                        measurement: Some(0),
                        hazard: false,
                    }],
                },
                KernelRow {
                    state: state_one,
                    command: 0,
                    outcomes: vec![TransitionOutcome {
                        probability: Rational::one(),
                        next_state: state_one,
                        realized_action: 2,
                        readback: Some(2),
                        measurement: Some(1),
                        hazard: false,
                    }],
                },
            ],
        }
    }

    #[test]
    fn repeated_command_uses_the_persistent_apparatus_state() {
        let kernel = backlash_kernel();
        let first = filter_one_step(
            &kernel,
            &kernel.initial_belief,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Value(1),
                measurement: ObservationField::Value(0),
            },
        )
        .unwrap();
        let next_belief = first.belief_for_next_step().unwrap();
        assert_eq!(next_belief[0].state.apparatus_state, 1);

        let second = filter_one_step(
            &kernel,
            &next_belief,
            0,
            PublicObservation {
                hazard: false,
                readback: ObservationField::Value(2),
                measurement: ObservationField::Value(1),
            },
        )
        .unwrap();
        assert_eq!(second.joint_posterior.len(), 1);
        assert_eq!(second.joint_posterior[0].realized_action, 2);
        assert_eq!(second.joint_posterior[0].next_state.apparatus_state, 1);
    }

    #[test]
    fn exact_zero_likelihood_is_not_renormalized() {
        let kernel = xor_kernel();
        let result = filter_one_step(
            &kernel,
            &kernel.initial_belief,
            0,
            PublicObservation {
                hazard: true,
                readback: ObservationField::Any,
                measurement: ObservationField::Any,
            },
        );
        assert_eq!(result, Err(FilterError::ZeroLikelihood));
    }

    #[test]
    fn terminal_hazard_cannot_be_used_as_a_next_step_belief() {
        let mut kernel = xor_kernel();
        for row in &mut kernel.rows {
            for outcome in &mut row.outcomes {
                outcome.hazard = true;
                outcome.measurement = None;
            }
        }
        let result = filter_one_step(
            &kernel,
            &kernel.initial_belief,
            0,
            PublicObservation {
                hazard: true,
                readback: ObservationField::Any,
                measurement: ObservationField::Any,
            },
        )
        .unwrap();
        assert_eq!(result.belief_for_next_step(), Err(FilterError::TerminalHazard));
    }

    #[test]
    fn malformed_kernel_probability_mass_fails_closed() {
        let mut kernel = xor_kernel();
        kernel.rows[0].outcomes[0].probability = Rational::new(1, 3).unwrap();
        assert!(kernel.validate().is_err());
    }

    #[test]
    fn rational_arithmetic_is_exact_and_checked() {
        let half = Rational::new(1, 2).unwrap();
        let quarter = Rational::new(1, 4).unwrap();
        assert_eq!(half.checked_add(quarter).unwrap(), Rational::new(3, 4).unwrap());
        assert_eq!(half.checked_mul(half).unwrap(), quarter);
        assert!(Rational::new(1, 0).is_err());
    }
}
