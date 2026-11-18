//! Finite-state value-of-information benchmark for budgeted HoloFrac experiments.
//!
//! The fixture distinguishes information gain from decision value. It uses
//! exact finite probability sums rather than Monte Carlo so the oracle is
//! deterministic and independently checkable.

pub const VOI_SCHEMA_VERSION: u32 = 1;
pub const VOI_FIXTURE_GENERATOR: &str = "holofrac-decision-value-fixture-v0";

#[derive(Clone, Debug, PartialEq)]
pub struct DesignDecision {
    pub decision_id: String,
    /// Utility by latent world state, normalized to [0, 1].
    pub utility_by_world: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentAction {
    pub experiment_id: String,
    pub cost: f64,
    /// Likelihood matrix: likelihoods[world_index][observation_index].
    pub likelihoods: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecisionProblem {
    pub world_state_ids: Vec<String>,
    pub prior: Vec<f64>,
    pub decisions: Vec<DesignDecision>,
    pub experiments: Vec<ExperimentAction>,
    pub experiment_budget: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExperimentSelectionPolicy {
    NoTest,
    Random,
    MaximumInformationGain,
    ExpectedDecisionValue,
}

impl ExperimentSelectionPolicy {
    pub const ALL: [Self; 4] = [
        Self::NoTest,
        Self::Random,
        Self::MaximumInformationGain,
        Self::ExpectedDecisionValue,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NoTest => "no_test",
            Self::Random => "random",
            Self::MaximumInformationGain => "max_information_gain",
            Self::ExpectedDecisionValue => "expected_decision_value",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentMetrics {
    pub experiment_id: String,
    pub cost: f64,
    pub mutual_information_bits: f64,
    pub baseline_expected_value: f64,
    pub expected_value_after_observation: f64,
    pub expected_value_improvement: f64,
    pub net_expected_decision_value: f64,
    pub expected_decision_regret: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyOutcome {
    pub policy: ExperimentSelectionPolicy,
    pub seed: u64,
    pub selected_experiment_id: Option<String>,
    pub mutual_information_bits: f64,
    pub experiment_cost: f64,
    pub baseline_expected_value: f64,
    pub expected_value_after_observation: f64,
    pub expected_value_improvement: f64,
    pub net_expected_decision_value: f64,
    pub expected_decision_regret: f64,
    pub decision_without_measurement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VoiError {
    EmptyWorldStateSet,
    EmptyDecisionSet,
    EmptyExperimentSet,
    DuplicateWorldState(String),
    DuplicateDecision(String),
    DuplicateExperiment(String),
    InvalidPrior,
    InvalidBudget,
    InvalidUtility(String),
    InvalidExperimentCost(String),
    InvalidLikelihoodShape(String),
    InvalidLikelihood(String),
    EmptyObservationSet(String),
}

impl std::fmt::Display for VoiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyWorldStateSet => f.write_str("at least one latent world state is required"),
            Self::EmptyDecisionSet => f.write_str("at least one design decision is required"),
            Self::EmptyExperimentSet => f.write_str("at least one experiment is required"),
            Self::DuplicateWorldState(id) => write!(f, "duplicate world-state ID: {id}"),
            Self::DuplicateDecision(id) => write!(f, "duplicate decision ID: {id}"),
            Self::DuplicateExperiment(id) => write!(f, "duplicate experiment ID: {id}"),
            Self::InvalidPrior => {
                f.write_str("prior probabilities must be finite, non-negative and sum to one")
            }
            Self::InvalidBudget => f.write_str("experiment budget must be finite and non-negative"),
            Self::InvalidUtility(id) => {
                write!(f, "decision utility vector is invalid for decision {id}")
            }
            Self::InvalidExperimentCost(id) => {
                write!(f, "experiment cost is invalid for experiment {id}")
            }
            Self::InvalidLikelihoodShape(id) => {
                write!(f, "experiment likelihood matrix has inconsistent shape: {id}")
            }
            Self::InvalidLikelihood(id) => {
                write!(f, "experiment likelihoods must be finite probabilities summing to one: {id}")
            }
            Self::EmptyObservationSet(id) => {
                write!(f, "experiment has no observation outcomes: {id}")
            }
        }
    }
}

impl std::error::Error for VoiError {}

#[derive(Clone, Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }
}

fn max_expected_decision_value(
    prior: &[f64],
    decisions: &[DesignDecision],
) -> (String, f64) {
    let mut best_id = String::new();
    let mut best_value = f64::NEG_INFINITY;

    for decision in decisions {
        let expected_value = prior
            .iter()
            .zip(decision.utility_by_world.iter())
            .map(|(probability, utility)| probability * utility)
            .sum::<f64>();

        if expected_value > best_value
            || (expected_value == best_value && decision.decision_id < best_id)
        {
            best_id = decision.decision_id.clone();
            best_value = expected_value;
        }
    }

    (best_id, best_value)
}

fn entropy_bits(probabilities: &[f64]) -> f64 {
    probabilities
        .iter()
        .filter(|probability| **probability > 0.0)
        .map(|probability| -probability * probability.log2())
        .sum()
}

impl DecisionProblem {
    pub fn validate(&self) -> Result<(), VoiError> {
        if self.world_state_ids.is_empty() {
            return Err(VoiError::EmptyWorldStateSet);
        }
        if self.decisions.is_empty() {
            return Err(VoiError::EmptyDecisionSet);
        }
        if self.experiments.is_empty() {
            return Err(VoiError::EmptyExperimentSet);
        }
        if !self.experiment_budget.is_finite() || self.experiment_budget < 0.0 {
            return Err(VoiError::InvalidBudget);
        }
        if self.prior.len() != self.world_state_ids.len()
            || self.prior.iter().any(|value| !value.is_finite() || *value < 0.0)
            || (self.prior.iter().sum::<f64>() - 1.0).abs() > 1.0e-9
        {
            return Err(VoiError::InvalidPrior);
        }

        let mut world_ids = std::collections::BTreeSet::new();
        for id in &self.world_state_ids {
            if !world_ids.insert(id) {
                return Err(VoiError::DuplicateWorldState(id.clone()));
            }
        }

        let mut decision_ids = std::collections::BTreeSet::new();
        for decision in &self.decisions {
            if !decision_ids.insert(&decision.decision_id) {
                return Err(VoiError::DuplicateDecision(decision.decision_id.clone()));
            }
            if decision.utility_by_world.len() != self.world_state_ids.len()
                || decision
                    .utility_by_world
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            {
                return Err(VoiError::InvalidUtility(decision.decision_id.clone()));
            }
        }

        let mut experiment_ids = std::collections::BTreeSet::new();
        for experiment in &self.experiments {
            if !experiment_ids.insert(&experiment.experiment_id) {
                return Err(VoiError::DuplicateExperiment(experiment.experiment_id.clone()));
            }
            if !experiment.cost.is_finite() || experiment.cost < 0.0 {
                return Err(VoiError::InvalidExperimentCost(
                    experiment.experiment_id.clone(),
                ));
            }
            if experiment.likelihoods.len() != self.world_state_ids.len() {
                return Err(VoiError::InvalidLikelihoodShape(
                    experiment.experiment_id.clone(),
                ));
            }
            let observation_count = experiment
                .likelihoods
                .first()
                .map_or(0, Vec::len);
            if observation_count == 0 {
                return Err(VoiError::EmptyObservationSet(
                    experiment.experiment_id.clone(),
                ));
            }
            for row in &experiment.likelihoods {
                if row.len() != observation_count {
                    return Err(VoiError::InvalidLikelihoodShape(
                        experiment.experiment_id.clone(),
                    ));
                }
                if row
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                    || (row.iter().sum::<f64>() - 1.0).abs() > 1.0e-9
                {
                    return Err(VoiError::InvalidLikelihood(
                        experiment.experiment_id.clone(),
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn baseline_value(&self) -> (String, f64) {
        max_expected_decision_value(&self.prior, &self.decisions)
    }

    pub fn perfect_information_value(&self) -> f64 {
        self.world_state_ids
            .iter()
            .enumerate()
            .map(|(world_index, _)| {
                let best_utility = self
                    .decisions
                    .iter()
                    .map(|decision| decision.utility_by_world[world_index])
                    .fold(f64::NEG_INFINITY, f64::max);
                self.prior[world_index] * best_utility
            })
            .sum()
    }

    pub fn evaluate_experiment(
        &self,
        experiment_index: usize,
    ) -> Result<ExperimentMetrics, VoiError> {
        self.validate()?;
        let experiment = self
            .experiments
            .get(experiment_index)
            .ok_or_else(|| VoiError::InvalidLikelihoodShape("unknown experiment".to_owned()))?;

        let (decision_without_measurement, baseline_expected_value) = self.baseline_value();
        let observation_count = experiment.likelihoods[0].len();
        let mut observation_probabilities = vec![0.0; observation_count];
        let mut joint_probabilities =
            vec![vec![0.0; observation_count]; self.world_state_ids.len()];

        for world_index in 0..self.world_state_ids.len() {
            for observation_index in 0..observation_count {
                let joint = self.prior[world_index]
                    * experiment.likelihoods[world_index][observation_index];
                joint_probabilities[world_index][observation_index] = joint;
                observation_probabilities[observation_index] += joint;
            }
        }

        let mut expected_value_after_observation = 0.0;
        for observation_index in 0..observation_count {
            let observation_probability = observation_probabilities[observation_index];
            if observation_probability <= 0.0 {
                continue;
            }

            let posterior: Vec<f64> = (0..self.world_state_ids.len())
                .map(|world_index| {
                    joint_probabilities[world_index][observation_index] / observation_probability
                })
                .collect();
            let (_, posterior_best_value) =
                max_expected_decision_value(&posterior, &self.decisions);
            expected_value_after_observation +=
                observation_probability * posterior_best_value;
        }

        let conditional_entropy_bits = self
            .prior
            .iter()
            .zip(experiment.likelihoods.iter())
            .map(|(probability, likelihoods)| probability * entropy_bits(likelihoods))
            .sum::<f64>();
        let mutual_information_bits =
            (entropy_bits(&observation_probabilities) - conditional_entropy_bits).max(0.0);
        let expected_value_improvement =
            expected_value_after_observation - baseline_expected_value;
        let net_expected_decision_value =
            expected_value_improvement - experiment.cost;
        let expected_decision_regret =
            self.perfect_information_value() - expected_value_after_observation;

        Ok(ExperimentMetrics {
            experiment_id: experiment.experiment_id.clone(),
            cost: experiment.cost,
            mutual_information_bits,
            baseline_expected_value,
            expected_value_after_observation,
            expected_value_improvement,
            net_expected_decision_value,
            expected_decision_regret,
        })
    }

    pub fn run_policy(
        &self,
        policy: ExperimentSelectionPolicy,
        seed: u64,
    ) -> Result<PolicyOutcome, VoiError> {
        self.validate()?;
        let (decision_without_measurement, baseline_value) = self.baseline_value();
        let eligible: Vec<usize> = self
            .experiments
            .iter()
            .enumerate()
            .filter_map(|(index, experiment)| {
                (experiment.cost <= self.experiment_budget).then_some(index)
            })
            .collect();

        let selected_index = match policy {
            ExperimentSelectionPolicy::NoTest => None,
            ExperimentSelectionPolicy::Random => {
                if eligible.is_empty() {
                    None
                } else {
                    let mut rng = DeterministicRng::new(seed);
                    let choice = (rng.next_u64() % eligible.len() as u64) as usize;
                    Some(eligible[choice])
                }
            }
            ExperimentSelectionPolicy::MaximumInformationGain => {
                let mut best: Option<(usize, f64)> = None;
                for index in eligible.iter().copied() {
                    let metrics = self.evaluate_experiment(index)?;
                    let replace = best.is_none_or(|(best_index, best_information)| {
                        metrics.mutual_information_bits > best_information
                            || (metrics.mutual_information_bits == best_information
                                && self.experiments[index].experiment_id
                                    < self.experiments[best_index].experiment_id)
                    });
                    if replace {
                        best = Some((index, metrics.mutual_information_bits));
                    }
                }
                best.map(|(index, _)| index)
            }
            ExperimentSelectionPolicy::ExpectedDecisionValue => {
                let mut best: Option<(usize, f64)> = None;
                for index in eligible.iter().copied() {
                    let metrics = self.evaluate_experiment(index)?;
                    let replace = best.is_none_or(|(best_index, best_value)| {
                        metrics.net_expected_decision_value > best_value
                            || (metrics.net_expected_decision_value == best_value
                                && self.experiments[index].experiment_id
                                    < self.experiments[best_index].experiment_id)
                    });
                    if replace {
                        best = Some((index, metrics.net_expected_decision_value));
                    }
                }
                best.filter(|(_, value)| *value > 0.0)
                    .map(|(index, _)| index)
            }
        };

        let perfect_information_value = self.perfect_information_value();
        if let Some(index) = selected_index {
            let metrics = self.evaluate_experiment(index)?;
            Ok(PolicyOutcome {
                policy,
                seed,
                selected_experiment_id: Some(metrics.experiment_id),
                mutual_information_bits: metrics.mutual_information_bits,
                experiment_cost: metrics.cost,
                baseline_expected_value: metrics.baseline_expected_value,
                expected_value_after_observation: metrics.expected_value_after_observation,
                expected_value_improvement: metrics.expected_value_improvement,
                net_expected_decision_value: metrics.net_expected_decision_value,
                expected_decision_regret: metrics.expected_decision_regret,
                decision_without_measurement,
            })
        } else {
            Ok(PolicyOutcome {
                policy,
                seed,
                selected_experiment_id: None,
                mutual_information_bits: 0.0,
                experiment_cost: 0.0,
                baseline_expected_value: baseline_value,
                expected_value_after_observation: baseline_value,
                expected_value_improvement: 0.0,
                net_expected_decision_value: 0.0,
                expected_decision_regret: perfect_information_value - baseline_value,
                decision_without_measurement,
            })
        }
    }
}

/// Synthetic four-world problem with a decision-relevant A factor and a
/// high-entropy but decision-irrelevant B factor.
pub fn synthetic_voi_problem() -> DecisionProblem {
    let world_state_ids = vec![
        "a0_b0".to_owned(),
        "a0_b1".to_owned(),
        "a1_b0".to_owned(),
        "a1_b1".to_owned(),
    ];
    let prior = vec![0.25; 4];

    let decisions = vec![
        DesignDecision {
            decision_id: "choose_a0".to_owned(),
            utility_by_world: vec![1.0, 1.0, 0.0, 0.0],
        },
        DesignDecision {
            decision_id: "choose_a1".to_owned(),
            utility_by_world: vec![0.0, 0.0, 1.0, 1.0],
        },
    ];

    let experiments = vec![
        ExperimentAction {
            experiment_id: "measure_a_70pct".to_owned(),
            cost: 0.02,
            likelihoods: vec![
                vec![0.7, 0.3],
                vec![0.7, 0.3],
                vec![0.3, 0.7],
                vec![0.3, 0.7],
            ],
        },
        ExperimentAction {
            experiment_id: "measure_b_exact".to_owned(),
            cost: 0.01,
            likelihoods: vec![
                vec![1.0, 0.0],
                vec![0.0, 1.0],
                vec![1.0, 0.0],
                vec![0.0, 1.0],
            ],
        },
        ExperimentAction {
            experiment_id: "measure_a_90pct".to_owned(),
            cost: 0.15,
            likelihoods: vec![
                vec![0.9, 0.1],
                vec![0.9, 0.1],
                vec![0.1, 0.9],
                vec![0.1, 0.9],
            ],
        },
    ];

    DecisionProblem {
        world_state_ids,
        prior,
        decisions,
        experiments,
        experiment_budget: 0.2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_value_selects_relevant_test_while_information_gain_selects_irrelevant_entropy() {
        let problem = synthetic_voi_problem();

        let information = problem
            .run_policy(ExperimentSelectionPolicy::MaximumInformationGain, 7)
            .unwrap();
        let decision_value = problem
            .run_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 7)
            .unwrap();

        assert_eq!(
            information.selected_experiment_id.as_deref(),
            Some("measure_b_exact")
        );
        assert_eq!(
            decision_value.selected_experiment_id.as_deref(),
            Some("measure_a_90pct")
        );
        assert!(
            information.mutual_information_bits > decision_value.mutual_information_bits
        );
        assert!((information.mutual_information_bits - 1.0).abs() < 1.0e-12);
        assert!(
            (decision_value.mutual_information_bits - 1.0 - 0.9_f64.log2() * 0.9
                - 0.1_f64.log2() * 0.1)
                .abs()
                < 1.0e-12
        );
        assert!(
            decision_value.net_expected_decision_value
                > information.net_expected_decision_value
        );
        assert!(
            decision_value.expected_decision_regret
                < information.expected_decision_regret
        );
    }

    #[test]
    fn no_test_is_preferred_when_every_experiment_has_negative_net_value() {
        let mut problem = synthetic_voi_problem();
        for experiment in &mut problem.experiments {
            experiment.cost = 0.9;
        }
        problem.experiment_budget = 1.0;

        let result = problem
            .run_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 1)
            .unwrap();
        assert_eq!(result.selected_experiment_id, None);
        assert_eq!(result.experiment_cost, 0.0);
        assert!(result.expected_decision_regret > 0.0);
    }

    #[test]
    fn random_selection_is_reproducible_for_a_fixed_seed() {
        let problem = synthetic_voi_problem();
        let first = problem
            .run_policy(ExperimentSelectionPolicy::Random, 123)
            .unwrap();
        let second = problem
            .run_policy(ExperimentSelectionPolicy::Random, 123)
            .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn budget_excludes_unaffordable_experiments() {
        let mut problem = synthetic_voi_problem();
        problem.experiment_budget = 0.05;

        let result = problem
            .run_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 1)
            .unwrap();
        assert_eq!(
            result.selected_experiment_id.as_deref(),
            Some("measure_a_70pct")
        );
    }

    #[test]
    fn likelihood_rows_and_priors_must_be_valid() {
        let mut problem = synthetic_voi_problem();
        problem.prior[0] = 0.5;

        assert_eq!(problem.validate(), Err(VoiError::InvalidPrior));
    }

    #[test]
    fn exact_expected_values_match_the_finite_oracle() {
        let problem = synthetic_voi_problem();
        let (_, baseline) = problem.baseline_value();
        let perfect = problem.perfect_information_value();
        let test_a = problem.evaluate_experiment(2).unwrap();
        let test_b = problem.evaluate_experiment(1).unwrap();

        assert!((baseline - 0.5).abs() < 1.0e-12);
        assert!((perfect - 1.0).abs() < 1.0e-12);
        assert!((test_a.baseline_expected_value - 0.5).abs() < 1.0e-12);
        assert!((test_a.expected_value_after_observation - 0.9).abs() < 1.0e-12);
        assert!((test_a.expected_value_improvement - 0.4).abs() < 1.0e-12);
        assert!((test_a.expected_value_improvement - 0.4).abs() < 1.0e-12);
        assert!((test_a.net_expected_decision_value - 0.25).abs() < 1.0e-12);
        assert!((test_b.expected_value_after_observation - 0.5).abs() < 1.0e-12);
        assert!((test_b.net_expected_decision_value + 0.01).abs() < 1.0e-12);
    }
}
