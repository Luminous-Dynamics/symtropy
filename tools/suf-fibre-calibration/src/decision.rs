// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Discrete expected-information and decision-value calculations.

use core::fmt;

/// Errors in a discrete measurement/decision model.
#[derive(Clone, Debug, PartialEq)]
pub enum DecisionError {
    /// Empty or duplicate identifier.
    InvalidIdentifier(&'static str),
    /// Inconsistent matrix or vector dimensions.
    DimensionMismatch(&'static str),
    /// Invalid or non-normalized probability.
    InvalidProbability(&'static str),
    /// Negative or non-finite loss/cost.
    InvalidLossOrCost(&'static str),
    /// A mathematically impossible negative EVSI beyond numeric tolerance.
    NegativeEvsi,
}

impl fmt::Display for DecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier(name) => write!(f, "invalid identifier: {name}"),
            Self::DimensionMismatch(name) => write!(f, "dimension mismatch: {name}"),
            Self::InvalidProbability(name) => write!(f, "invalid probability: {name}"),
            Self::InvalidLossOrCost(name) => write!(f, "invalid loss or cost: {name}"),
            Self::NegativeEvsi => write!(
                f,
                "expected value of sample information is materially negative"
            ),
        }
    }
}

impl std::error::Error for DecisionError {}

/// Result type for discrete decision calculations.
pub type DecisionResult<T> = Result<T, DecisionError>;

/// Frozen hypotheses, prior and decision losses.
/// Losses are indexed by decision, then hypothesis.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionProblem {
    /// Stable hypothesis identifiers.
    hypothesis_ids: Vec<String>,
    /// Prior probability of each hypothesis.
    prior: Vec<f64>,
    /// Stable candidate decision identifiers.
    decision_ids: Vec<String>,
    /// Loss matrix indexed by decision, then hypothesis.
    loss_by_decision: Vec<Vec<f64>>,
    /// Declared unit for all decision losses.
    loss_unit: String,
}

impl DecisionProblem {
    /// Construct a validated decision problem.
    pub fn new(
        hypothesis_ids: Vec<String>,
        prior: Vec<f64>,
        decision_ids: Vec<String>,
        loss_by_decision: Vec<Vec<f64>>,
        loss_unit: String,
    ) -> DecisionResult<Self> {
        validate_unique_ids(&hypothesis_ids, "hypothesis_ids")?;
        validate_unique_ids(&decision_ids, "decision_ids")?;
        validate_distribution(&prior, "prior")?;
        if hypothesis_ids.len() != prior.len() {
            return Err(DecisionError::DimensionMismatch("hypothesis_ids/prior"));
        }
        if decision_ids.len() != loss_by_decision.len() || decision_ids.is_empty() {
            return Err(DecisionError::DimensionMismatch(
                "decision_ids/loss_by_decision",
            ));
        }
        if loss_unit.trim().is_empty() {
            return Err(DecisionError::InvalidIdentifier("loss_unit"));
        }
        for row in &loss_by_decision {
            if row.len() != hypothesis_ids.len() {
                return Err(DecisionError::DimensionMismatch("loss matrix row"));
            }
            if row.iter().any(|loss| !loss.is_finite() || *loss < 0.0) {
                return Err(DecisionError::InvalidLossOrCost("loss matrix"));
            }
        }
        Ok(Self {
            hypothesis_ids,
            prior,
            decision_ids,
            loss_by_decision,
            loss_unit,
        })
    }

    fn minimum_expected_loss(&self, probabilities: &[f64]) -> DecisionResult<f64> {
        if probabilities.len() != self.hypothesis_ids.len() {
            return Err(DecisionError::DimensionMismatch("posterior hypotheses"));
        }
        let minimum = self
            .loss_by_decision
            .iter()
            .map(|row| {
                row.iter()
                    .zip(probabilities)
                    .map(|(loss, probability)| *loss * *probability)
                    .sum::<f64>()
            })
            .fold(f64::INFINITY, f64::min);
        if minimum.is_finite() {
            Ok(minimum)
        } else {
            Err(DecisionError::InvalidLossOrCost("expected loss"))
        }
    }
}

/// Candidate measurement and P(outcome | hypothesis) likelihood matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementAction {
    /// Stable measurement action identifier.
    action_id: String,
    /// Stable measurement outcome identifiers.
    outcome_ids: Vec<String>,
    /// Conditional probabilities indexed by hypothesis, then outcome.
    likelihoods: Vec<Vec<f64>>,
    /// Acquisition cost in cost_unit.
    acquisition_cost: f64,
    /// Unit for acquisition cost.
    cost_unit: String,
}

impl MeasurementAction {
    /// Construct a validated measurement action.
    pub fn new(
        action_id: String,
        outcome_ids: Vec<String>,
        likelihoods: Vec<Vec<f64>>,
        acquisition_cost: f64,
        cost_unit: String,
        hypothesis_count: usize,
    ) -> DecisionResult<Self> {
        if action_id.trim().is_empty() {
            return Err(DecisionError::InvalidIdentifier("action_id"));
        }
        validate_unique_ids(&outcome_ids, "outcome_ids")?;
        if likelihoods.len() != hypothesis_count || hypothesis_count == 0 {
            return Err(DecisionError::DimensionMismatch(
                "likelihood hypothesis rows",
            ));
        }
        if outcome_ids.is_empty() {
            return Err(DecisionError::DimensionMismatch("outcome_ids"));
        }
        if cost_unit.trim().is_empty() {
            return Err(DecisionError::InvalidIdentifier("cost_unit"));
        }
        if !acquisition_cost.is_finite() || acquisition_cost < 0.0 {
            return Err(DecisionError::InvalidLossOrCost("acquisition_cost"));
        }
        for row in &likelihoods {
            if row.len() != outcome_ids.len() {
                return Err(DecisionError::DimensionMismatch(
                    "likelihood outcome columns",
                ));
            }
            validate_distribution(row, "likelihood row")?;
        }
        Ok(Self {
            action_id,
            outcome_ids,
            likelihoods,
            acquisition_cost,
            cost_unit,
        })
    }
}

/// Cost-based disposition. This is not authorization to acquire a measurement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionDisposition {
    /// Comparable net value is positive.
    Acquire,
    /// Comparable net value is not positive.
    DoNotAcquire,
    /// Decision loss and action cost use incomparable units.
    NoCostBasedRecommendation,
}

/// Separate information-theoretic and decision-theoretic metrics.
#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementAssessment {
    /// Candidate action identifier.
    pub action_id: String,
    /// Minimum expected loss before measurement.
    pub prior_minimum_expected_loss: f64,
    /// Expected minimum loss after observing the measurement.
    pub expected_posterior_minimum_loss: f64,
    /// Expected value of sample information, in loss_unit.
    pub expected_value_of_sample_information: f64,
    /// Expected information gain, in bits.
    pub expected_information_gain_bits: f64,
    /// Net value only when cost_unit equals loss_unit.
    pub net_value: Option<f64>,
    /// Cost-based disposition, never an authorization decision.
    pub disposition: AcquisitionDisposition,
}

impl DecisionProblem {
    /// Evaluate expected information gain and expected value of sample
    /// information separately. Does not select or authorize a real measurement.
    pub fn assess(&self, action: &MeasurementAction) -> DecisionResult<MeasurementAssessment> {
        if action.likelihoods.len() != self.hypothesis_ids.len()
            || action
                .likelihoods
                .iter()
                .any(|row| row.len() != action.outcome_ids.len())
        {
            return Err(DecisionError::DimensionMismatch(
                "action/problem dimensions",
            ));
        }

        let prior_minimum_expected_loss = self.minimum_expected_loss(&self.prior)?;
        let prior_entropy = entropy_bits(&self.prior);
        let mut expected_posterior_minimum_loss = 0.0;
        let mut expected_posterior_entropy = 0.0;

        for (outcome_index, _) in action.outcome_ids.iter().enumerate() {
            let weighted: Vec<f64> = self
                .prior
                .iter()
                .enumerate()
                .map(|(h, prior)| *prior * action.likelihoods[h][outcome_index])
                .collect();
            let outcome_probability: f64 = weighted.iter().sum();
            if outcome_probability <= f64::EPSILON {
                continue;
            }
            let posterior: Vec<f64> = weighted
                .iter()
                .map(|probability| probability / outcome_probability)
                .collect();
            expected_posterior_minimum_loss +=
                outcome_probability * self.minimum_expected_loss(&posterior)?;
            expected_posterior_entropy += outcome_probability * entropy_bits(&posterior);
        }

        if !expected_posterior_minimum_loss.is_finite() || !expected_posterior_entropy.is_finite() {
            return Err(DecisionError::InvalidLossOrCost("posterior expectations"));
        }
        let raw_evsi = prior_minimum_expected_loss - expected_posterior_minimum_loss;
        if raw_evsi < -1e-9 {
            return Err(DecisionError::NegativeEvsi);
        }
        let expected_value_of_sample_information = raw_evsi.max(0.0);
        let raw_eig = prior_entropy - expected_posterior_entropy;
        let expected_information_gain_bits = if raw_eig < 0.0 && raw_eig > -1e-9 {
            0.0
        } else {
            raw_eig
        };
        if !expected_information_gain_bits.is_finite() || expected_information_gain_bits < 0.0 {
            return Err(DecisionError::InvalidProbability(
                "expected information gain",
            ));
        }

        let (net_value, disposition) = if action.cost_unit == self.loss_unit {
            let net = expected_value_of_sample_information - action.acquisition_cost;
            let choice = if net > 0.0 {
                AcquisitionDisposition::Acquire
            } else {
                AcquisitionDisposition::DoNotAcquire
            };
            (Some(net), choice)
        } else {
            (None, AcquisitionDisposition::NoCostBasedRecommendation)
        };

        Ok(MeasurementAssessment {
            action_id: action.action_id.clone(),
            prior_minimum_expected_loss,
            expected_posterior_minimum_loss,
            expected_value_of_sample_information,
            expected_information_gain_bits,
            net_value,
            disposition,
        })
    }
}

fn validate_unique_ids(ids: &[String], field: &'static str) -> DecisionResult<()> {
    if ids.is_empty() || ids.iter().any(|id| id.trim().is_empty()) {
        return Err(DecisionError::InvalidIdentifier(field));
    }
    for (index, id) in ids.iter().enumerate() {
        if ids[index + 1..].iter().any(|other| other == id) {
            return Err(DecisionError::InvalidIdentifier(field));
        }
    }
    Ok(())
}

fn validate_distribution(values: &[f64], field: &'static str) -> DecisionResult<()> {
    if values.is_empty()
        || values
            .iter()
            .any(|p| !p.is_finite() || *p < 0.0 || *p > 1.0)
    {
        return Err(DecisionError::InvalidProbability(field));
    }
    let total: f64 = values.iter().sum();
    if !total.is_finite() || (total - 1.0).abs() > 1e-9 {
        return Err(DecisionError::InvalidProbability(field));
    }
    Ok(())
}

fn entropy_bits(probabilities: &[f64]) -> f64 {
    probabilities
        .iter()
        .filter(|p| **p > 0.0)
        .map(|p| -*p * p.log2())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binary_problem(loss_unit: &str, decision_irrelevant: bool) -> DecisionProblem {
        let loss_by_decision = if decision_irrelevant {
            vec![vec![0.0, 0.0], vec![1.0, 1.0]]
        } else {
            vec![vec![0.0, 2.0], vec![2.0, 0.0]]
        };
        DecisionProblem::new(
            vec!["fault".to_owned(), "no_fault".to_owned()],
            vec![0.5, 0.5],
            vec!["dispatch".to_owned(), "do_nothing".to_owned()],
            loss_by_decision,
            loss_unit.to_owned(),
        )
        .expect("valid decision problem")
    }

    fn perfect_sensor(cost: f64, cost_unit: &str) -> MeasurementAction {
        MeasurementAction::new(
            "survey-check".to_owned(),
            vec!["fault_seen".to_owned(), "quiet".to_owned()],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            cost,
            cost_unit.to_owned(),
            2,
        )
        .expect("valid measurement action")
    }

    #[test]
    fn perfect_measurement_has_one_bit_and_positive_evsi() {
        let assessment = binary_problem("USD", false)
            .assess(&perfect_sensor(0.2, "USD"))
            .expect("assessment");
        assert!((assessment.expected_information_gain_bits - 1.0).abs() < 1e-9);
        assert!((assessment.expected_value_of_sample_information - 1.0).abs() < 1e-9);
        assert!((assessment.net_value.expect("comparable units") - 0.8).abs() < 1e-9);
        assert_eq!(assessment.disposition, AcquisitionDisposition::Acquire);
    }

    #[test]
    fn uninformative_measurement_has_zero_information_and_decision_value() {
        let action = MeasurementAction::new(
            "unhelpful".to_owned(),
            vec!["a".to_owned(), "b".to_owned()],
            vec![vec![0.5, 0.5], vec![0.5, 0.5]],
            0.1,
            "USD".to_owned(),
            2,
        )
        .expect("valid action");
        let assessment = binary_problem("USD", false)
            .assess(&action)
            .expect("assessment");
        assert!(assessment.expected_information_gain_bits.abs() < 1e-9);
        assert!(assessment.expected_value_of_sample_information.abs() < 1e-9);
        assert_eq!(assessment.disposition, AcquisitionDisposition::DoNotAcquire);
    }

    #[test]
    fn information_gain_can_be_positive_while_evsi_is_zero() {
        let assessment = binary_problem("USD", true)
            .assess(&perfect_sensor(0.1, "USD"))
            .expect("assessment");
        assert!((assessment.expected_information_gain_bits - 1.0).abs() < 1e-9);
        assert!(assessment.expected_value_of_sample_information.abs() < 1e-9);
        assert!(assessment.net_value.expect("comparable units") < 0.0);
        assert_eq!(assessment.disposition, AcquisitionDisposition::DoNotAcquire);
    }

    #[test]
    fn incomparable_units_suppress_net_value_and_recommendation() {
        let assessment = binary_problem("outage_minutes", false)
            .assess(&perfect_sensor(0.1, "USD"))
            .expect("assessment");
        assert!(assessment.net_value.is_none());
        assert_eq!(
            assessment.disposition,
            AcquisitionDisposition::NoCostBasedRecommendation
        );
        assert!((assessment.expected_information_gain_bits - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rejects_non_normalized_prior() {
        assert_eq!(
            DecisionProblem::new(
                vec!["h1".to_owned(), "h2".to_owned()],
                vec![0.4, 0.4],
                vec!["d1".to_owned()],
                vec![vec![0.0, 0.0]],
                "USD".to_owned(),
            ),
            Err(DecisionError::InvalidProbability("prior"))
        );
    }

    #[test]
    fn rejects_non_normalized_likelihood_rows() {
        assert_eq!(
            MeasurementAction::new(
                "bad".to_owned(),
                vec!["o1".to_owned(), "o2".to_owned()],
                vec![vec![0.2, 0.2], vec![0.5, 0.5]],
                0.0,
                "USD".to_owned(),
                2,
            ),
            Err(DecisionError::InvalidProbability("likelihood row"))
        );
    }

    #[test]
    fn rejects_negative_measurement_cost() {
        assert_eq!(
            MeasurementAction::new(
                "bad".to_owned(),
                vec!["o1".to_owned()],
                vec![vec![1.0], vec![1.0]],
                -1.0,
                "USD".to_owned(),
                2,
            ),
            Err(DecisionError::InvalidLossOrCost("acquisition_cost"))
        );
    }

    #[test]
    fn rejects_duplicate_outcomes() {
        assert_eq!(
            MeasurementAction::new(
                "bad".to_owned(),
                vec!["same".to_owned(), "same".to_owned()],
                vec![vec![0.5, 0.5], vec![0.5, 0.5]],
                0.0,
                "USD".to_owned(),
                2,
            ),
            Err(DecisionError::InvalidIdentifier("outcome_ids"))
        );
    }
}
