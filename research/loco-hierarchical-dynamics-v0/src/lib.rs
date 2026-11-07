//! Hierarchical inference for a single scalar locomotor dynamical parameter.
//!
//! The observed value is a session-level estimate (for example, log angular
//! frequency) with a supplied measurement variance. Repeated steps within a
//! session must be summarized upstream; they are not independent animals.
//!
//! Model:
//!   mu ~ Normal(mu0, v0)
//!   theta_i | mu ~ Normal(mu, tau_animal^2)
//!   y_ij | theta_i ~ Normal(theta_i, tau_session^2 + measurement_var_ij)
//!
//! Animal-effect variances are compared on an explicit finite prior grid.
//! This is a conjugate reference model, not a full nonlinear oscillator fit.

use std::collections::BTreeMap;

const LOG_TWO_PI: f64 = 1.837_877_066_409_345_3;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionEstimate {
    pub animal_id: String,
    pub session_id: String,
    /// One session-level estimate of an unbounded parameterization.
    /// Positive parameters should normally be supplied on a log scale.
    pub value: f64,
    /// Posterior/estimation variance from the upstream session fit.
    pub measurement_variance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VarianceCandidate {
    /// Animal random-intercept variance. Zero is the pooled/no-individual-effect model.
    pub animal_effect_variance: f64,
    /// Prior mass for this candidate; normalized internally.
    pub prior_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HierarchicalConfig {
    pub population_prior_mean: f64,
    pub population_prior_variance: f64,
    /// Independent session-to-session variance, shared across animals.
    pub session_effect_variance: f64,
    /// Finite, preregistered hyperparameter grid, including zero for a pooled null.
    pub variance_candidates: Vec<VarianceCandidate>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalSummary {
    pub mean: f64,
    pub variance: f64,
}

impl NormalSummary {
    pub fn standard_deviation(&self) -> f64 {
        self.variance.sqrt()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HyperparameterPosterior {
    pub animal_effect_variance: f64,
    pub posterior_probability: f64,
    pub log_marginal_likelihood: f64,
    pub population: NormalSummary,
    /// Marginal animal parameter posteriors under this variance candidate.
    pub animals: BTreeMap<String, NormalSummary>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HierarchicalFit {
    pub components: Vec<HyperparameterPosterior>,
    pub animal_count: usize,
    pub session_count: usize,
    pub session_effect_variance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictiveSummary {
    pub mean: f64,
    pub variance: f64,
}

impl PredictiveSummary {
    pub fn standard_deviation(&self) -> f64 {
        self.variance.sqrt()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HierarchicalError {
    NoObservations,
    EmptyAnimalId,
    EmptySessionId,
    DuplicateAnimalSession {
        animal_id: String,
        session_id: String,
    },
    NonFiniteValue,
    InvalidMeasurementVariance,
    InvalidPopulationPrior,
    InvalidSessionVariance,
    NoVarianceCandidates,
    InvalidVarianceCandidate,
    InvalidCandidatePrior,
    NumericalFailure,
    UnknownAnimal(String),
    InvalidPredictionVariance,
}

#[derive(Debug, Clone)]
struct GroupSufficientStatistics {
    animal_id: String,
    session_values: Vec<(f64, f64)>,
    sum_precision: f64,
    sum_precision_value: f64,
    sum_precision_value_squared: f64,
    sum_log_measurement_variance: f64,
}

impl HierarchicalConfig {
    pub fn validate(&self) -> Result<(), HierarchicalError> {
        if !self.population_prior_mean.is_finite()
            || !self.population_prior_variance.is_finite()
            || self.population_prior_variance <= 0.0
        {
            return Err(HierarchicalError::InvalidPopulationPrior);
        }
        if !self.session_effect_variance.is_finite() || self.session_effect_variance < 0.0 {
            return Err(HierarchicalError::InvalidSessionVariance);
        }
        if self.variance_candidates.is_empty() {
            return Err(HierarchicalError::NoVarianceCandidates);
        }

        let mut seen = Vec::with_capacity(self.variance_candidates.len());
        let mut prior_total = 0.0;
        for candidate in &self.variance_candidates {
            if !candidate.animal_effect_variance.is_finite()
                || candidate.animal_effect_variance < 0.0
            {
                return Err(HierarchicalError::InvalidVarianceCandidate);
            }
            if !candidate.prior_weight.is_finite() || candidate.prior_weight <= 0.0 {
                return Err(HierarchicalError::InvalidCandidatePrior);
            }
            if seen.contains(&candidate.animal_effect_variance) {
                return Err(HierarchicalError::InvalidVarianceCandidate);
            }
            seen.push(candidate.animal_effect_variance);
            prior_total += candidate.prior_weight;
        }
        if !prior_total.is_finite() || prior_total <= 0.0 {
            return Err(HierarchicalError::InvalidCandidatePrior);
        }
        if !seen.iter().any(|variance| *variance == 0.0) {
            return Err(HierarchicalError::InvalidVarianceCandidate);
        }
        Ok(())
    }
}

pub fn fit_hierarchical_parameter(
    observations: &[SessionEstimate],
    config: &HierarchicalConfig,
) -> Result<HierarchicalFit, HierarchicalError> {
    config.validate()?;
    if observations.is_empty() {
        return Err(HierarchicalError::NoObservations);
    }

    let groups = build_groups(observations, config.session_effect_variance)?;
    let animal_count = groups.len();
    let session_count = observations.len();

    let prior_total: f64 = config
        .variance_candidates
        .iter()
        .map(|candidate| candidate.prior_weight)
        .sum();

    let mut unnormalized = Vec::with_capacity(config.variance_candidates.len());
    for candidate in &config.variance_candidates {
        let component = fit_component(
            &groups,
            config.population_prior_mean,
            config.population_prior_variance,
            candidate.animal_effect_variance,
        )?;
        let normalized_prior = candidate.prior_weight / prior_total;
        let log_weight = normalized_prior.ln() + component.log_marginal_likelihood;
        unnormalized.push((component, log_weight));
    }

    let log_normalizer = log_sum_exp(
        &unnormalized
            .iter()
            .map(|(_, log_weight)| *log_weight)
            .collect::<Vec<_>>(),
    )?;

    let components = unnormalized
        .into_iter()
        .map(|(mut component, log_weight)| {
            component.posterior_probability = (log_weight - log_normalizer).exp();
            component
        })
        .collect();

    Ok(HierarchicalFit {
        components,
        animal_count,
        session_count,
        session_effect_variance: config.session_effect_variance,
    })
}

impl HierarchicalFit {
    /// Posterior probability assigned to non-zero between-animal variance.
    pub fn probability_of_individual_dynamics(&self) -> f64 {
        self.components
            .iter()
            .filter(|component| component.animal_effect_variance > 0.0)
            .map(|component| component.posterior_probability)
            .sum()
    }

    /// Model-averaged population parameter posterior.
    pub fn population_posterior(&self) -> NormalSummary {
        mixture_summary(self.components.iter().map(|component| {
            (
                component.posterior_probability,
                component.population.mean,
                component.population.variance,
            )
        }))
    }

    /// Model-averaged animal parameter posterior, including uncertainty in the
    /// population mean and the between-animal variance model.
    pub fn animal_posterior(&self, animal_id: &str) -> Result<NormalSummary, HierarchicalError> {
        if !self
            .components
            .iter()
            .any(|component| component.animals.contains_key(animal_id))
        {
            return Err(HierarchicalError::UnknownAnimal(animal_id.to_owned()));
        }
        Ok(mixture_summary(self.components.iter().filter_map(
            |component| {
                component.animals.get(animal_id).map(|posterior| {
                    (
                        component.posterior_probability,
                        posterior.mean,
                        posterior.variance,
                    )
                })
            },
        )))
    }

    /// Predict a new session for an animal observed in the training data.
    /// The new session effect and measurement uncertainty are included.
    pub fn predict_existing_animal_session(
        &self,
        animal_id: &str,
        measurement_variance: f64,
    ) -> Result<PredictiveSummary, HierarchicalError> {
        validate_prediction_variance(measurement_variance)?;
        if !self
            .components
            .iter()
            .any(|component| component.animals.contains_key(animal_id))
        {
            return Err(HierarchicalError::UnknownAnimal(animal_id.to_owned()));
        }
        let summary = mixture_summary(self.components.iter().filter_map(|component| {
            component.animals.get(animal_id).map(|posterior| {
                (
                    component.posterior_probability,
                    posterior.mean,
                    posterior.variance + self.session_effect_variance + measurement_variance,
                )
            })
        }));
        Ok(PredictiveSummary {
            mean: summary.mean,
            variance: summary.variance,
        })
    }

    /// Cold-start prediction for an entirely unseen animal and one future
    /// session. No animal-specific observations are used.
    pub fn predict_unseen_animal_session(
        &self,
        measurement_variance: f64,
    ) -> Result<PredictiveSummary, HierarchicalError> {
        validate_prediction_variance(measurement_variance)?;
        let summary = mixture_summary(self.components.iter().map(|component| {
            (
                component.posterior_probability,
                component.population.mean,
                component.population.variance
                    + component.animal_effect_variance
                    + self.session_effect_variance
                    + measurement_variance,
            )
        }));
        Ok(PredictiveSummary {
            mean: summary.mean,
            variance: summary.variance,
        })
    }
}

fn validate_prediction_variance(value: f64) -> Result<(), HierarchicalError> {
    if !value.is_finite() || value < 0.0 {
        return Err(HierarchicalError::InvalidPredictionVariance);
    }
    Ok(())
}

fn build_groups(
    observations: &[SessionEstimate],
    session_effect_variance: f64,
) -> Result<Vec<GroupSufficientStatistics>, HierarchicalError> {
    let mut grouped: BTreeMap<String, BTreeMap<String, (f64, f64)>> = BTreeMap::new();

    for observation in observations {
        if observation.animal_id.trim().is_empty() {
            return Err(HierarchicalError::EmptyAnimalId);
        }
        if observation.session_id.trim().is_empty() {
            return Err(HierarchicalError::EmptySessionId);
        }
        if !observation.value.is_finite() {
            return Err(HierarchicalError::NonFiniteValue);
        }
        if !observation.measurement_variance.is_finite() || observation.measurement_variance <= 0.0
        {
            return Err(HierarchicalError::InvalidMeasurementVariance);
        }

        let animal = grouped.entry(observation.animal_id.clone()).or_default();
        if animal
            .insert(
                observation.session_id.clone(),
                (observation.value, observation.measurement_variance),
            )
            .is_some()
        {
            return Err(HierarchicalError::DuplicateAnimalSession {
                animal_id: observation.animal_id.clone(),
                session_id: observation.session_id.clone(),
            });
        }
    }

    let mut groups = Vec::with_capacity(grouped.len());
    for (animal_id, sessions) in grouped {
        let mut sum_precision = 0.0;
        let mut sum_precision_value = 0.0;
        let mut sum_precision_value_squared = 0.0;
        let mut sum_log_measurement_variance = 0.0;
        let mut session_values = Vec::with_capacity(sessions.len());

        for (_, (value, measurement_variance)) in sessions {
            let variance = session_effect_variance + measurement_variance;
            let precision = 1.0 / variance;
            sum_precision += precision;
            sum_precision_value += precision * value;
            sum_precision_value_squared += precision * value * value;
            sum_log_measurement_variance += variance.ln();
            session_values.push((value, variance));
        }

        if !sum_precision.is_finite()
            || !sum_precision_value.is_finite()
            || !sum_precision_value_squared.is_finite()
            || !sum_log_measurement_variance.is_finite()
        {
            return Err(HierarchicalError::NumericalFailure);
        }

        groups.push(GroupSufficientStatistics {
            animal_id,
            session_values,
            sum_precision,
            sum_precision_value,
            sum_precision_value_squared,
            sum_log_measurement_variance,
        });
    }
    Ok(groups)
}

fn fit_component(
    groups: &[GroupSufficientStatistics],
    population_prior_mean: f64,
    population_prior_variance: f64,
    animal_effect_variance: f64,
) -> Result<HyperparameterPosterior, HierarchicalError> {
    let mut population_precision = 1.0 / population_prior_variance;
    let mut population_precision_mean = population_prior_mean / population_prior_variance;
    let mut total_quadratic = 0.0;
    let mut total_log_determinant = 0.0;

    for group in groups {
        let denominator = 1.0 + animal_effect_variance * group.sum_precision;
        let group_weight = group.sum_precision / denominator;
        let group_information = group.sum_precision_value / denominator;
        let group_quadratic = group.sum_precision_value_squared
            - animal_effect_variance * group.sum_precision_value.powi(2) / denominator;
        let group_log_determinant = group.sum_log_measurement_variance + denominator.ln();

        population_precision += group_weight;
        population_precision_mean += group_information;
        total_quadratic += group_quadratic;
        total_log_determinant += group_log_determinant;

        if !denominator.is_finite()
            || denominator <= 0.0
            || !group_weight.is_finite()
            || !group_information.is_finite()
            || !group_quadratic.is_finite()
            || !group_log_determinant.is_finite()
        {
            return Err(HierarchicalError::NumericalFailure);
        }
    }

    let population_variance = 1.0 / population_precision;
    let population_mean = population_variance * population_precision_mean;
    let integrated_quadratic = total_quadratic
        + population_prior_mean.powi(2) / population_prior_variance
        - population_precision_mean.powi(2) / population_precision;

    let observation_count: usize = groups.iter().map(|group| group.session_values.len()).sum();
    let log_marginal_likelihood = -0.5
        * (observation_count as f64 * LOG_TWO_PI
            + total_log_determinant
            + (population_prior_variance * population_precision).ln()
            + integrated_quadratic);

    if !population_mean.is_finite()
        || !population_variance.is_finite()
        || population_variance <= 0.0
        || !log_marginal_likelihood.is_finite()
    {
        return Err(HierarchicalError::NumericalFailure);
    }

    let mut animals = BTreeMap::new();
    for group in groups {
        let posterior = if animal_effect_variance == 0.0 {
            NormalSummary {
                mean: population_mean,
                variance: population_variance,
            }
        } else {
            let conditional_precision = 1.0 / animal_effect_variance + group.sum_precision;
            let conditional_variance = 1.0 / conditional_precision;
            let population_gain = conditional_variance / animal_effect_variance;
            let conditional_data_term = conditional_variance * group.sum_precision_value;
            let mean = population_gain * population_mean + conditional_data_term;
            let variance = conditional_variance + population_gain.powi(2) * population_variance;
            if !mean.is_finite() || !variance.is_finite() || variance <= 0.0 {
                return Err(HierarchicalError::NumericalFailure);
            }
            NormalSummary { mean, variance }
        };
        animals.insert(group.animal_id.clone(), posterior);
    }

    Ok(HyperparameterPosterior {
        animal_effect_variance,
        posterior_probability: 0.0,
        log_marginal_likelihood,
        population: NormalSummary {
            mean: population_mean,
            variance: population_variance,
        },
        animals,
    })
}

fn mixture_summary(components: impl Iterator<Item = (f64, f64, f64)>) -> NormalSummary {
    let values: Vec<_> = components.filter(|(weight, _, _)| *weight > 0.0).collect();
    let total_weight: f64 = values.iter().map(|(weight, _, _)| *weight).sum();
    let mean = values
        .iter()
        .map(|(weight, value, _)| weight * value)
        .sum::<f64>()
        / total_weight;
    let second_moment = values
        .iter()
        .map(|(weight, value, variance)| weight * (variance + value * value))
        .sum::<f64>()
        / total_weight;
    NormalSummary {
        mean,
        variance: (second_moment - mean * mean).max(0.0),
    }
}

fn log_sum_exp(values: &[f64]) -> Result<f64, HierarchicalError> {
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !maximum.is_finite() {
        return Err(HierarchicalError::NumericalFailure);
    }
    let sum = values
        .iter()
        .map(|value| (value - maximum).exp())
        .sum::<f64>();
    let result = maximum + sum.ln();
    if !result.is_finite() {
        return Err(HierarchicalError::NumericalFailure);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(candidates: Vec<VarianceCandidate>) -> HierarchicalConfig {
        HierarchicalConfig {
            population_prior_mean: 0.0,
            population_prior_variance: 100.0,
            session_effect_variance: 0.01,
            variance_candidates: candidates,
        }
    }

    fn candidate(variance: f64, weight: f64) -> VarianceCandidate {
        VarianceCandidate {
            animal_effect_variance: variance,
            prior_weight: weight,
        }
    }

    fn rows(values: &[(&str, &[f64])], measurement_variance: f64) -> Vec<SessionEstimate> {
        let mut output = Vec::new();
        for (animal, sessions) in values {
            for (index, value) in sessions.iter().enumerate() {
                output.push(SessionEstimate {
                    animal_id: (*animal).to_owned(),
                    session_id: format!("session-{index}"),
                    value: *value,
                    measurement_variance,
                });
            }
        }
        output
    }

    #[test]
    fn one_animal_random_effect_posterior_matches_a_gaussian_known_answer() {
        let data = vec![
            SessionEstimate {
                animal_id: "animal-1".into(),
                session_id: "session-1".into(),
                value: 1.0,
                measurement_variance: 1.0,
            },
            SessionEstimate {
                animal_id: "animal-1".into(),
                session_id: "session-2".into(),
                value: 3.0,
                measurement_variance: 1.0,
            },
        ];
        let mut cfg = config(vec![candidate(1.0, 1.0)]);
        cfg.population_prior_mean = 0.0;
        cfg.population_prior_variance = 4.0;
        cfg.session_effect_variance = 0.0;
        let fit = fit_hierarchical_parameter(&data, &cfg).unwrap();
        let component = &fit.components[0];

        assert!((component.population.mean - 16.0 / 11.0).abs() < 1.0e-12);
        assert!((component.population.variance - 12.0 / 11.0).abs() < 1.0e-12);
        assert!((component.log_marginal_likelihood - (-4.400_461_066_444_894)).abs() < 1.0e-9);

        let animal = fit.animal_posterior("animal-1").unwrap();
        assert!((animal.mean - 20.0 / 11.0).abs() < 1.0e-12);
        assert!((animal.variance - 5.0 / 11.0).abs() < 1.0e-12);
    }

    #[test]
    fn population_and_individual_effect_probabilities_are_not_confused() {
        let data = rows(
            &[("a", &[0.0, 0.0]), ("b", &[1.0, 1.0]), ("c", &[-1.0, -1.0])],
            0.001,
        );
        let fit = fit_hierarchical_parameter(
            &data,
            &config(vec![candidate(0.0, 1.0), candidate(1.0, 1.0)]),
        )
        .unwrap();
        let probability = fit.probability_of_individual_dynamics();
        assert!(probability > 0.99);
        assert!(
            (fit.components
                .iter()
                .map(|item| item.posterior_probability)
                .sum::<f64>()
                - 1.0)
                .abs()
                < 1.0e-12
        );
        assert!(fit.animal_posterior("a").unwrap().mean < fit.population_posterior().mean);
    }

    #[test]
    fn identical_animal_dynamics_support_the_pooled_null() {
        let data = rows(
            &[
                ("a", &[1.00, 1.01, 0.99, 1.00]),
                ("b", &[1.01, 1.00, 0.99, 1.00]),
                ("c", &[0.99, 1.00, 1.01, 1.00]),
                ("d", &[1.00, 0.99, 1.00, 1.01]),
                ("e", &[1.01, 1.00, 1.00, 0.99]),
                ("f", &[0.99, 1.01, 1.00, 1.00]),
            ],
            0.0025,
        );
        let fit = fit_hierarchical_parameter(
            &data,
            &config(vec![candidate(0.0, 1.0), candidate(0.25, 1.0)]),
        )
        .unwrap();
        assert!(fit.probability_of_individual_dynamics() < 0.05);
        assert!(fit.population_posterior().variance > 0.0);
    }

    #[test]
    fn stable_individual_differences_support_nonzero_animal_variance() {
        let data = rows(
            &[
                ("a", &[0.0, 0.01, -0.01, 0.0]),
                ("b", &[1.0, 1.01, 0.99, 1.0]),
                ("c", &[-1.0, -0.99, -1.01, -1.0]),
                ("d", &[0.5, 0.51, 0.49, 0.5]),
                ("e", &[-0.5, -0.49, -0.51, -0.5]),
                ("f", &[1.5, 1.51, 1.49, 1.5]),
                ("g", &[-1.5, -1.49, -1.51, -1.5]),
            ],
            0.0025,
        );
        let fit = fit_hierarchical_parameter(
            &data,
            &config(vec![
                candidate(0.0, 1.0),
                candidate(0.5, 2.0),
                candidate(2.0, 1.0),
            ]),
        )
        .unwrap();
        assert!(fit.probability_of_individual_dynamics() > 0.95);
        let a = fit.animal_posterior("a").unwrap();
        let b = fit.animal_posterior("b").unwrap();
        assert!(b.mean > a.mean + 0.5);
    }

    #[test]
    fn animal_estimates_are_shrunk_toward_the_population() {
        let data = rows(
            &[("a", &[0.0]), ("b", &[0.1]), ("c", &[0.0]), ("d", &[0.0])],
            1.0,
        );
        let fit = fit_hierarchical_parameter(
            &data,
            &config(vec![candidate(0.0, 1.0), candidate(0.1, 10.0)]),
        )
        .unwrap();
        let population = fit.population_posterior();
        let animal = fit.animal_posterior("b").unwrap();
        assert!(animal.mean.abs() < 0.1);
        assert!(animal.variance > 0.0);
        assert!(population.variance > 0.0);
    }

    #[test]
    fn unseen_animal_prediction_includes_population_and_animal_uncertainty() {
        let data = rows(&[("a", &[0.0, 0.1]), ("b", &[0.2, 0.1])], 0.05);
        let fit = fit_hierarchical_parameter(
            &data,
            &config(vec![candidate(0.0, 1.0), candidate(0.5, 1.0)]),
        )
        .unwrap();
        let known = fit.predict_existing_animal_session("a", 0.01).unwrap();
        let unseen = fit.predict_unseen_animal_session(0.01).unwrap();
        assert!(unseen.variance > known.variance);
    }

    #[test]
    fn input_order_does_not_change_posterior() {
        let data = rows(
            &[("a", &[0.0, 0.2]), ("b", &[0.5, 0.7]), ("c", &[-0.4, -0.2])],
            0.1,
        );
        let mut reversed = data.clone();
        reversed.reverse();
        let cfg = config(vec![candidate(0.0, 1.0), candidate(0.2, 2.0)]);
        let left = fit_hierarchical_parameter(&data, &cfg).unwrap();
        let right = fit_hierarchical_parameter(&reversed, &cfg).unwrap();
        assert_eq!(left.components, right.components);
        assert_eq!(left.population_posterior(), right.population_posterior());
    }

    #[test]
    fn duplicate_animal_session_is_rejected() {
        let data = vec![
            SessionEstimate {
                animal_id: "a".into(),
                session_id: "s1".into(),
                value: 0.1,
                measurement_variance: 0.1,
            },
            SessionEstimate {
                animal_id: "a".into(),
                session_id: "s1".into(),
                value: 0.2,
                measurement_variance: 0.1,
            },
        ];
        assert!(matches!(
            fit_hierarchical_parameter(
                &data,
                &config(vec![candidate(0.0, 1.0), candidate(0.2, 1.0)])
            ),
            Err(HierarchicalError::DuplicateAnimalSession { .. })
        ));
    }

    #[test]
    fn invalid_candidate_grid_without_pooled_null_is_rejected() {
        let data = rows(&[("a", &[0.1]), ("b", &[0.2])], 0.1);
        assert_eq!(
            fit_hierarchical_parameter(&data, &config(vec![candidate(0.2, 1.0)])),
            Err(HierarchicalError::InvalidVarianceCandidate)
        );
    }

    #[test]
    fn session_effects_and_measurement_uncertainty_are_both_in_predictions() {
        let data = rows(&[("a", &[0.1]), ("b", &[0.2])], 0.1);
        let mut cfg = config(vec![candidate(0.0, 1.0), candidate(0.5, 1.0)]);
        cfg.session_effect_variance = 0.3;
        let fit = fit_hierarchical_parameter(&data, &cfg).unwrap();
        let prediction = fit.predict_existing_animal_session("a", 0.2).unwrap();
        assert!(prediction.variance > 0.5);
    }
}
