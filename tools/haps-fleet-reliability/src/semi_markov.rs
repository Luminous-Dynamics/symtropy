//! Independent equation-level replication of the HAPS k-out-of-n semi-Markov model.
//!
//! This implementation evaluates the transition kernel and sojourn-time
//! integrals separately. It intentionally does not copy the paper's printed
//! closed-form sojourn expressions; those are audited in the accompanying
//! replication note because the displayed expressions appear dimensionally
//! inconsistent with the integrals in Eqs. (14)-(17).
//!
//! Scope: independent exponential component failures, a single
//! hypoexponential repair clock in intermediate states, and a Weibull repair
//! clock in the all-system-failed state. Common-cause events, weather,
//! maintenance logistics and network dependencies are not represented here.

use std::fmt;

const TOLERANCE: f64 = 1.0e-10;
const MAX_COMPONENTS: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub struct SemiMarkovConfig {
    pub n: usize,
    pub k: usize,
    pub component_failure_rate: f64,
    pub initial_multi_failure_probabilities: Vec<f64>,
    /// Sum mu1 + mu2; the paper fixes mu1=mu2/2.
    pub repair_rate_sum: f64,
    /// Weibull scale beta in F(t)=1-exp(-(beta*t)^shape).
    pub system_repair_weibull_scale: f64,
    pub system_repair_weibull_shape: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemiMarkovError(String);

impl SemiMarkovError {
    fn new(message: impl Into<String>) -> Self { Self(message.into()) }
}

impl fmt::Display for SemiMarkovError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for SemiMarkovError {}

#[derive(Debug, Clone, PartialEq)]
pub struct SemiMarkovReport {
    pub n: usize,
    pub k: usize,
    pub component_failure_rates: Vec<f64>,
    pub hypoexponential_repair_rates: [f64; 2],
    pub transition_matrix: Vec<Vec<f64>>,
    pub mean_sojourn_times: Vec<f64>,
    pub embedded_stationary_probabilities: Vec<f64>,
    pub time_stationary_probabilities: Vec<f64>,
    pub steady_state_availability: f64,
    pub expected_transient_state_visits: Vec<f64>,
    pub mean_time_to_system_failure: f64,
}

impl SemiMarkovConfig {
    pub fn validate(&self) -> Result<(), SemiMarkovError> {
        if self.k < 2 || self.n < self.k || self.n > MAX_COMPONENTS {
            return Err(SemiMarkovError::new("require 2 <= k <= n <= MAX_COMPONENTS"));
        }
        if self.initial_multi_failure_probabilities.len() != self.k - 1 {
            return Err(SemiMarkovError::new(
                "initial multi-failure probability vector must have k-1 entries",
            ));
        }
        if !positive_finite(self.component_failure_rate)
            || !positive_finite(self.repair_rate_sum)
            || !positive_finite(self.system_repair_weibull_scale)
            || !self.system_repair_weibull_shape.is_finite()
            || self.system_repair_weibull_shape <= 1.0
        {
            return Err(SemiMarkovError::new(
                "failure, repair and Weibull scale must be positive finite; Weibull shape must exceed 1",
            ));
        }
        let mut sum = 0.0;
        for probability in &self.initial_multi_failure_probabilities {
            if !probability.is_finite() || *probability <= 0.0 || *probability > 1.0 {
                return Err(SemiMarkovError::new(
                    "initial multi-failure probabilities must be finite and in (0,1]",
                ));
            }
            sum += probability;
        }
        if (sum - 1.0).abs() > TOLERANCE {
            return Err(SemiMarkovError::new(
                "initial multi-failure probabilities must sum to one",
            ));
        }
        Ok(())
    }
}

/// Evaluate the embedded transition chain, stationary time occupancy and MTTF.
///
/// States count failed platforms. States 0..k-1 are functional and state k
/// is the all-system-failed state. For stationary availability, state k is
/// regenerative through its Weibull system-repair sojourn and transition to k-1.
pub fn evaluate(config: &SemiMarkovConfig) -> Result<SemiMarkovReport, SemiMarkovError> {
    config.validate()?;
    let state_count = config.k + 1;
    let mu1 = config.repair_rate_sum / 3.0;
    let mu2 = 2.0 * config.repair_rate_sum / 3.0;
    let failure_rates = (0..config.k)
        .map(|failed| (config.n - failed) as f64 * config.component_failure_rate)
        .collect::<Vec<_>>();

    let mut transition_matrix = vec![vec![0.0; state_count]; state_count];
    let c_sum = config.initial_multi_failure_probabilities.iter().sum::<f64>();
    for (offset, probability) in config.initial_multi_failure_probabilities.iter().enumerate() {
        transition_matrix[0][offset + 1] = probability / c_sum;
    }

    for failed in 1..config.k {
        let lambda = failure_rates[failed];
        let denominator = (lambda + mu1) * (lambda + mu2);
        let p_failure = lambda * (lambda + mu1 + mu2) / denominator;
        let p_repair = mu1 * mu2 / denominator;
        transition_matrix[failed][failed + 1] = p_failure;
        transition_matrix[failed][failed - 1] = p_repair;
    }
    transition_matrix[config.k][config.k - 1] = 1.0;

    for (state, row) in transition_matrix.iter().enumerate() {
        if (row.iter().sum::<f64>() - 1.0).abs() > TOLERANCE {
            return Err(SemiMarkovError::new(format!(
                "embedded transition row {state} does not sum to one"
            )));
        }
    }

    // Direct integration of Eqs. (13)-(17). In an intermediate state,
    // E[min(T_failure,T_repair)] is the survival-product integral. It has
    // units of time and therefore is not multiplied by lambda.
    let mut mean_sojourn_times = vec![0.0; state_count];
    mean_sojourn_times[0] = 1.0 / (failure_rates[0] * c_sum);
    for failed in 1..config.k {
        let lambda = failure_rates[failed];
        mean_sojourn_times[failed] =
            (lambda + mu1 + mu2) / ((lambda + mu1) * (lambda + mu2));
    }
    mean_sojourn_times[config.k] = weibull_mean(
        config.system_repair_weibull_scale,
        config.system_repair_weibull_shape,
    )?;

    let embedded_stationary_probabilities = stationary_distribution(&transition_matrix)?;
    let mut time_stationary_probabilities = embedded_stationary_probabilities
        .iter()
        .zip(&mean_sojourn_times)
        .map(|(probability, sojourn)| probability * sojourn)
        .collect::<Vec<_>>();
    let normalization = time_stationary_probabilities.iter().sum::<f64>();
    if !positive_finite(normalization) {
        return Err(SemiMarkovError::new("semi-Markov stationary normalization is invalid"));
    }
    for probability in &mut time_stationary_probabilities {
        *probability /= normalization;
    }

    let steady_state_availability = time_stationary_probabilities[..config.k]
        .iter()
        .sum::<f64>();
    let expected_transient_state_visits =
        expected_visits_before_absorption(&transition_matrix, config.k)?;
    let mean_time_to_system_failure = expected_transient_state_visits
        .iter()
        .zip(&mean_sojourn_times[..config.k])
        .map(|(visits, sojourn)| visits * sojourn)
        .sum::<f64>();

    if !steady_state_availability.is_finite()
        || !(0.0..=1.0).contains(&steady_state_availability)
        || !positive_finite(mean_time_to_system_failure)
    {
        return Err(SemiMarkovError::new(
            "semi-Markov evaluation produced invalid availability or MTTF",
        ));
    }

    Ok(SemiMarkovReport {
        n: config.n,
        k: config.k,
        component_failure_rates: failure_rates,
        hypoexponential_repair_rates: [mu1, mu2],
        transition_matrix,
        mean_sojourn_times,
        embedded_stationary_probabilities,
        time_stationary_probabilities,
        steady_state_availability,
        expected_transient_state_visits,
        mean_time_to_system_failure,
    })
}

fn weibull_mean(beta: f64, shape: f64) -> Result<f64, SemiMarkovError> {
    let value = gamma(1.0 + 1.0 / shape) / beta;
    if positive_finite(value) {
        Ok(value)
    } else {
        Err(SemiMarkovError::new("Weibull mean is not finite and positive"))
    }
}

/// Lanczos approximation for Gamma(z), for positive z.
fn gamma(argument: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_93,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if argument < 0.5 {
        return std::f64::consts::PI
            / ((std::f64::consts::PI * argument).sin() * gamma(1.0 - argument));
    }
    let z = argument - 1.0;
    let mut series = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        series += coefficient / (z + index as f64);
    }
    let t = z + 7.5;
    (2.0 * std::f64::consts::PI).sqrt() * t.powf(z + 0.5) * (-t).exp() * series
}

fn stationary_distribution(transition: &[Vec<f64>]) -> Result<Vec<f64>, SemiMarkovError> {
    let size = transition.len();
    let mut matrix = vec![vec![0.0; size]; size];
    let mut right_hand = vec![0.0; size];

    for equation in 0..(size - 1) {
        for variable in 0..size {
            matrix[equation][variable] =
                transition[variable][equation] - if variable == equation { 1.0 } else { 0.0 };
        }
    }
    for variable in 0..size {
        matrix[size - 1][variable] = 1.0;
    }
    right_hand[size - 1] = 1.0;

    let solution = solve_linear_system(matrix, right_hand)?;
    if solution
        .iter()
        .any(|probability| !probability.is_finite() || *probability < -TOLERANCE)
    {
        return Err(SemiMarkovError::new(
            "embedded stationary distribution contains invalid probabilities",
        ));
    }
    let clipped = solution
        .into_iter()
        .map(|probability| probability.max(0.0))
        .collect::<Vec<_>>();
    let total = clipped.iter().sum::<f64>();
    if (total - 1.0).abs() > TOLERANCE {
        return Err(SemiMarkovError::new(
            "embedded stationary distribution does not normalize",
        ));
    }
    Ok(clipped.into_iter().map(|probability| probability / total).collect())
}

fn expected_visits_before_absorption(
    transition: &[Vec<f64>],
    absorbing_state: usize,
) -> Result<Vec<f64>, SemiMarkovError> {
    let transient_count = absorbing_state;
    let mut matrix = vec![vec![0.0; transient_count]; transient_count];
    for row in 0..transient_count {
        for column in 0..transient_count {
            matrix[row][column] =
                if row == column { 1.0 } else { 0.0 } - transition[column][row];
        }
    }
    let mut right_hand = vec![0.0; transient_count];
    right_hand[0] = 1.0;
    let solution = solve_linear_system(matrix, right_hand)?;
    if solution
        .iter()
        .any(|visits| !visits.is_finite() || *visits < -TOLERANCE)
    {
        return Err(SemiMarkovError::new(
            "expected transient visits contain invalid values",
        ));
    }
    Ok(solution.into_iter().map(|visits| visits.max(0.0)).collect())
}

fn solve_linear_system(
    mut matrix: Vec<Vec<f64>>,
    mut right_hand: Vec<f64>,
) -> Result<Vec<f64>, SemiMarkovError> {
    let size = right_hand.len();
    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err(SemiMarkovError::new("linear system must be square"));
    }

    for pivot_column in 0..size {
        let mut pivot_row = pivot_column;
        for candidate in (pivot_column + 1)..size {
            if matrix[candidate][pivot_column].abs() > matrix[pivot_row][pivot_column].abs() {
                pivot_row = candidate;
            }
        }
        if matrix[pivot_row][pivot_column].abs() <= f64::EPSILON {
            return Err(SemiMarkovError::new("linear system is singular or ill-conditioned"));
        }
        matrix.swap(pivot_column, pivot_row);
        right_hand.swap(pivot_column, pivot_row);

        let pivot = matrix[pivot_column][pivot_column];
        for column in pivot_column..size {
            matrix[pivot_column][column] /= pivot;
        }
        right_hand[pivot_column] /= pivot;

        for row in 0..size {
            if row == pivot_column {
                continue;
            }
            let factor = matrix[row][pivot_column];
            for column in pivot_column..size {
                matrix[row][column] -= factor * matrix[pivot_column][column];
            }
            right_hand[row] -= factor * right_hand[pivot_column];
        }
    }
    Ok(right_hand)
}

fn positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference_config() -> SemiMarkovConfig {
        SemiMarkovConfig {
            n: 6,
            k: 3,
            component_failure_rate: 1.0,
            initial_multi_failure_probabilities: vec![0.5, 0.5],
            repair_rate_sum: 1.0,
            system_repair_weibull_scale: 1.0,
            system_repair_weibull_shape: 2.0,
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected:.15}, got {actual:.15}"
        );
    }

    #[test]
    fn reproduces_transition_kernel_from_paper_parameters() {
        let report = evaluate(&reference_config()).unwrap();
        close(report.transition_matrix[0][1], 0.5, 1.0e-12);
        close(report.transition_matrix[0][2], 0.5, 1.0e-12);
        close(report.transition_matrix[1][0], 1.0 / 136.0, 1.0e-12);
        close(report.transition_matrix[1][2], 135.0 / 136.0, 1.0e-12);
        close(report.transition_matrix[2][1], 1.0 / 91.0, 1.0e-12);
        close(report.transition_matrix[2][3], 90.0 / 91.0, 1.0e-12);
        close(report.transition_matrix[3][2], 1.0, 1.0e-12);
    }

    #[test]
    fn integrates_sojourn_times_directly_without_rate_multiplier() {
        let report = evaluate(&reference_config()).unwrap();
        close(report.mean_sojourn_times[0], 1.0 / 6.0, 1.0e-12);
        close(report.mean_sojourn_times[1], 27.0 / 136.0, 1.0e-12);
        close(report.mean_sojourn_times[2], 45.0 / 182.0, 1.0e-12);
        close(
            report.mean_sojourn_times[3],
            std::f64::consts::PI.sqrt() / 2.0,
            1.0e-12,
        );
    }

    #[test]
    fn reproduces_time_stationary_distribution_from_kernel_and_sojourns() {
        let report = evaluate(&reference_config()).unwrap();
        close(report.embedded_stationary_probabilities[0], 0.000_040_544_809_960_4, 1.0e-12);
        close(report.embedded_stationary_probabilities[1], 0.005_514_795_846_383_0, 1.0e-12);
        close(report.embedded_stationary_probabilities[2], 0.499_937_781_806_174_2, 1.0e-12);
        close(report.embedded_stationary_probabilities[3], 0.494_506_877_539_412_7, 1.0e-12);
        close(report.steady_state_availability, 0.221_530_944_400_130_2, 1.0e-12);
        close(
            report.time_stationary_probabilities.iter().sum::<f64>(),
            1.0,
            1.0e-12,
        );
    }

    #[test]
    fn computes_mttf_from_transient_fundamental_matrix() {
        let report = evaluate(&reference_config()).unwrap();
        close(report.expected_transient_state_visits[0], 1.003_772_037_720_377_2, 1.0e-12);
        close(report.expected_transient_state_visits[1], 0.007_462_074_620_746_2, 1.0e-12);
        close(report.expected_transient_state_visits[2], 0.000_082_000_820_008_2, 1.0e-12);
        close(report.mean_time_to_system_failure, 0.168_797_055_833_087_9, 1.0e-12);
    }

    #[test]
    fn weibull_mean_matches_shape_two_closed_form() {
        close(
            weibull_mean(1.0, 2.0).unwrap(),
            std::f64::consts::PI.sqrt() / 2.0,
            1.0e-12,
        );
    }

    #[test]
    fn every_transition_row_is_a_probability_distribution() {
        let report = evaluate(&reference_config()).unwrap();
        for row in &report.transition_matrix {
            close(row.iter().sum::<f64>(), 1.0, 1.0e-12);
        }
    }

    #[test]
    fn higher_component_failure_rate_reduces_availability() {
        let mut degraded = reference_config();
        degraded.component_failure_rate = 1.5;
        let baseline = evaluate(&reference_config()).unwrap();
        let worse = evaluate(&degraded).unwrap();
        assert!(worse.steady_state_availability < baseline.steady_state_availability);
    }

    #[test]
    fn invalid_probability_mass_is_rejected() {
        let mut config = reference_config();
        config.initial_multi_failure_probabilities = vec![0.4, 0.4];
        assert!(config.validate().is_err());
    }

    #[test]
    fn invalid_system_shape_is_rejected() {
        let mut config = reference_config();
        config.system_repair_weibull_shape = 1.0;
        assert!(config.validate().is_err());
    }
}
