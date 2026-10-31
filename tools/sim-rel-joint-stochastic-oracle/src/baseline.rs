//! Matched information-gain baselines for the exact joint stochastic model.

use super::{JointKernel, JointMass, Rational};
use std::collections::{BTreeMap, BTreeSet};

/// One-step expected information gain about the mechanism hypothesis.
///
/// Physical and sensor states are marginalized only after applying the full
/// joint transition-observation-hazard kernel. Hazard is treated as an observed
/// terminal event. The score is a myopic baseline and does not value actions
/// whose information appears only after a later intervention.
#[derive(Clone, Debug, PartialEq)]
pub struct OneStepEigScore {
    pub action_index: usize,
    pub action_id: String,
    pub expected_information_gain_nats: f64,
    pub information_gain_per_cost: f64,
    pub hazard_probability: Rational,
    pub outcome_count: usize,
}

/// Rank actions by one-step mechanism EIG per unit action cost.
///
/// This is a model-based comparator, not the exact finite-horizon optimizer.
/// It receives the current public posterior over joint hidden tuples and
/// integrates sensor and physical uncertainty using the same declared kernel.
pub fn rank_by_one_step_eig(
    kernel: &JointKernel,
    belief: &[JointMass],
) -> Result<Vec<OneStepEigScore>, String> {
    kernel.validate()?;
    validate_belief(kernel, belief)?;

    let mut prior_by_hypothesis = vec![Rational::zero(); kernel.hypothesis_ids.len()];
    for point in belief {
        prior_by_hypothesis[point.hypothesis] =
            prior_by_hypothesis[point.hypothesis].checked_add(point.mass)?;
    }
    let prior_entropy = entropy(&prior_by_hypothesis, Rational::one())?;

    let mut scores = Vec::with_capacity(kernel.action_ids.len());
    for action in 0..kernel.action_ids.len() {
        let mut event_by_hypothesis: BTreeMap<Option<usize>, Vec<Rational>> = BTreeMap::new();
        for point in belief {
            let row_index = kernel.row_index(
                point.hypothesis,
                point.physical_state,
                point.sensor_state,
                action,
            );
            for outcome in &kernel.rows[row_index].outcomes {
                let weighted_mass = point.mass.checked_mul(outcome.probability)?;
                let event = event_by_hypothesis
                    .entry(outcome.observation)
                    .or_insert_with(|| vec![Rational::zero(); kernel.hypothesis_ids.len()]);
                event[point.hypothesis] =
                    event[point.hypothesis].checked_add(weighted_mass)?;
            }
        }

        let mut total_event_mass = Rational::zero();
        let mut hazard_probability = Rational::zero();
        let mut expected_posterior_entropy = 0.0_f64;
        let outcome_count = event_by_hypothesis.len();
        for (observation, hypothesis_masses) in event_by_hypothesis {
            let event_mass = hypothesis_masses.iter().try_fold(
                Rational::zero(),
                |sum, mass| sum.checked_add(*mass),
            )?;
            total_event_mass = total_event_mass.checked_add(event_mass)?;
            if observation.is_none() {
                hazard_probability = hazard_probability.checked_add(event_mass)?;
            }
            expected_posterior_entropy +=
                to_f64(event_mass) * entropy(&hypothesis_masses, event_mass)?;
        }
        if total_event_mass != Rational::one() {
            return Err("one-step EIG event partition did not conserve probability mass".into());
        }

        let mut information_gain = prior_entropy - expected_posterior_entropy;
        if information_gain < -1e-10 {
            return Err("one-step EIG became materially negative; check model arithmetic".into());
        }
        information_gain = information_gain.max(0.0);
        let cost = kernel.action_costs[action] as f64;
        scores.push(OneStepEigScore {
            action_index: action,
            action_id: kernel.action_ids[action].clone(),
            expected_information_gain_nats: information_gain,
            information_gain_per_cost: information_gain / cost,
            hazard_probability,
            outcome_count,
        });
    }

    scores.sort_by(|left, right| {
        right
            .information_gain_per_cost
            .total_cmp(&left.information_gain_per_cost)
            .then_with(|| {
                right
                    .expected_information_gain_nats
                    .total_cmp(&left.expected_information_gain_nats)
            })
            .then_with(|| left.action_index.cmp(&right.action_index))
    });
    Ok(scores)
}

fn validate_belief(kernel: &JointKernel, belief: &[JointMass]) -> Result<(), String> {
    if belief.is_empty() || belief.len() > 65_536 {
        return Err("joint belief is empty or exceeds its tuple cap".into());
    }
    let mut seen = BTreeSet::new();
    let mut total = Rational::zero();
    for point in belief {
        if point.hypothesis >= kernel.hypothesis_ids.len()
            || point.physical_state >= kernel.physical_state_count
            || point.sensor_state >= kernel.sensor_state_count
            || point.mass.is_zero()
        {
            return Err("joint belief contains an invalid tuple or non-positive mass".into());
        }
        if !seen.insert((
            point.hypothesis,
            point.physical_state,
            point.sensor_state,
        )) {
            return Err("joint belief contains a duplicate tuple".into());
        }
        total = total.checked_add(point.mass)?;
    }
    if total != Rational::one() {
        return Err("joint belief probability mass must sum exactly to one".into());
    }
    Ok(())
}

fn entropy(masses: &[Rational], normalizer: Rational) -> Result<f64, String> {
    if normalizer.is_zero() {
        return Err("cannot calculate entropy for a zero-mass event".into());
    }
    let mut result = 0.0_f64;
    for mass in masses {
        if mass.is_zero() {
            continue;
        }
        let probability = to_f64(mass.checked_div(normalizer)?);
        result -= probability * probability.ln();
    }
    Ok(result)
}

fn to_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

