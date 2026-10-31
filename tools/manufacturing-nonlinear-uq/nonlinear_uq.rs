//! Nonlinear measurement-model uncertainty benchmark.
//!
//! Deterministic equal-probability quadrature is used here; this is NOT a
//! random Monte Carlo implementation. The benchmark compares first-order
//! linearization, second-order Taylor approximation (normal scalar inputs)
//! and distribution-propagated output mass.
//!
//! Build:
//!   rustc --edition=2024 -D warnings --test tools/manufacturing-nonlinear-uq/nonlinear_uq.rs -o /tmp/nonlinear-uq-tests
//!   /tmp/nonlinear-uq-tests
//!   rustc --edition=2024 -D warnings tools/manufacturing-nonlinear-uq/nonlinear_uq.rs -o /tmp/nonlinear-uq
//!   /tmp/nonlinear-uq --verify
//!   /tmp/nonlinear-uq --csv

const TAIL_PROBABILITY: f64 = 6.220960574271784e-16; // one-sided N(0,1) tail at 8 sigma
const TRUNCATED_MASS: f64 = 1.0 - 2.0 * TAIL_PROBABILITY;
const RESOLUTIONS: [usize; 4] = [256, 1024, 4096, 16384];
const ACCEPT_THRESHOLD: f64 = 0.95;
const REJECT_THRESHOLD: f64 = 0.05;
const MASS_TOLERANCE: f64 = 2.0e-12;

#[derive(Clone, Copy, Debug)]
struct GaussianComponent {
    weight: f64,
    mean: f64,
    sigma: f64,
}

#[derive(Clone, Debug)]
struct InputDistribution {
    components: Vec<GaussianComponent>,
}

impl InputDistribution {
    fn gaussian(mean: f64, sigma: f64) -> Self {
        Self {
            components: vec![GaussianComponent { weight: 1.0, mean, sigma }],
        }
    }

    fn bimodal() -> Self {
        Self {
            components: vec![
                GaussianComponent { weight: 0.5, mean: -1.0, sigma: 0.1 },
                GaussianComponent { weight: 0.5, mean: 1.0, sigma: 0.1 },
            ],
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.components.is_empty() {
            return Err("DISTRIBUTION_EMPTY".into());
        }
        let mut total = 0.0;
        for component in &self.components {
            if !component.weight.is_finite() || component.weight <= 0.0
                || !component.mean.is_finite()
                || !component.sigma.is_finite() || component.sigma <= 0.0 {
                return Err("DISTRIBUTION_COMPONENT_INVALID".into());
            }
            total += component.weight;
        }
        if !total.is_finite() || (total - 1.0).abs() > 1.0e-12 {
            return Err("DISTRIBUTION_MASS_NOT_NORMALIZED".into());
        }
        Ok(())
    }

    fn mean_variance(&self) -> Result<(f64, f64), String> {
        self.validate()?;
        let mean = self.components.iter().map(|c| c.weight * c.mean).sum::<f64>();
        let second = self.components.iter()
            .map(|c| c.weight * (c.sigma * c.sigma + c.mean * c.mean))
            .sum::<f64>();
        let variance = second - mean * mean;
        if !mean.is_finite() || !variance.is_finite() || variance <= 0.0 {
            return Err("DISTRIBUTION_MOMENTS_INVALID".into());
        }
        Ok((mean, variance))
    }

    fn sample_midpoint_quantiles(&self, n: usize) -> Result<(Vec<WeightedValue>, f64), String> {
        self.validate()?;
        if n < 16 {
            return Err("QUADRATURE_RESOLUTION_TOO_LOW".into());
        }
        let mut samples = Vec::with_capacity(n * self.components.len());
        let component_tail = 2.0 * TAIL_PROBABILITY;
        for component in &self.components {
            let weight = component.weight * TRUNCATED_MASS / n as f64;
            for i in 0..n {
                let p = TAIL_PROBABILITY
                    + (i as f64 + 0.5) * TRUNCATED_MASS / n as f64;
                let z = normal_quantile(p)?;
                let x = component.mean + component.sigma * z;
                if !x.is_finite() {
                    return Err("QUADRATURE_INPUT_NONFINITE".into());
                }
                samples.push(WeightedValue { value: x, weight });
            }
        }
        let omitted_tail = component_tail * self.components.iter().map(|c| c.weight).sum::<f64>();
        Ok((samples, omitted_tail))
    }
}

#[derive(Clone, Copy, Debug)]
struct WeightedValue {
    value: f64,
    weight: f64,
}

#[derive(Clone, Copy, Debug)]
enum Transform {
    Linear { scale: f64, offset: f64 },
    Square,
    Exponential,
    ReciprocalOneMinusX,
}

impl Transform {
    fn id(self) -> &'static str {
        match self {
            Self::Linear { .. } => "linear_2x_plus_1",
            Self::Square => "square",
            Self::Exponential => "exp",
            Self::ReciprocalOneMinusX => "reciprocal_1_over_1_minus_x",
        }
    }

    fn value(self, x: f64) -> Option<f64> {
        let result = match self {
            Self::Linear { scale, offset } => scale * x + offset,
            Self::Square => x * x,
            Self::Exponential => x.exp(),
            Self::ReciprocalOneMinusX => {
                if x >= 1.0 { return None; }
                1.0 / (1.0 - x)
            }
        };
        (result.is_finite()).then_some(result)
    }

    fn first_derivative(self, x: f64) -> Option<f64> {
        let result = match self {
            Self::Linear { scale, .. } => scale,
            Self::Square => 2.0 * x,
            Self::Exponential => x.exp(),
            Self::ReciprocalOneMinusX => {
                if x >= 1.0 { return None; }
                1.0 / (1.0 - x).powi(2)
            }
        };
        result.is_finite().then_some(result)
    }

    fn second_derivative(self, x: f64) -> Option<f64> {
        let result = match self {
            Self::Linear { .. } => 0.0,
            Self::Square => 2.0,
            Self::Exponential => x.exp(),
            Self::ReciprocalOneMinusX => {
                if x >= 1.0 { return None; }
                2.0 / (1.0 - x).powi(3)
            }
        };
        result.is_finite().then_some(result)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    Accept,
    Reject,
    Indeterminate,
    InvalidDomain,
}

impl Decision {
    fn code(self) -> &'static str {
        match self {
            Self::Accept => "ACCEPT",
            Self::Reject => "REJECT",
            Self::Indeterminate => "INDETERMINATE",
            Self::InvalidDomain => "INVALID_DOMAIN",
        }
    }
}

fn classify_probability(lower: f64, upper: f64) -> Decision {
    if lower >= ACCEPT_THRESHOLD {
        Decision::Accept
    } else if upper <= REJECT_THRESHOLD {
        Decision::Reject
    } else {
        Decision::Indeterminate
    }
}

#[derive(Clone, Debug)]
struct Propagated {
    resolution: usize,
    valid_mass: f64,
    invalid_domain_mass: f64,
    tail_mass: f64,
    unresolved_mass: f64,
    mean_conditional_valid: Option<f64>,
    variance_conditional_valid: Option<f64>,
    q025_conditional_valid: Option<f64>,
    q05_conditional_valid: Option<f64>,
    median_conditional_valid: Option<f64>,
    q95_conditional_valid: Option<f64>,
    q975_conditional_valid: Option<f64>,
    specification_probability_lower: f64,
    specification_probability_upper: f64,
    decision: Decision,
    histogram_masses: Vec<f64>,
}

#[derive(Clone, Debug)]
struct Scenario {
    id: &'static str,
    distribution: InputDistribution,
    transform: Transform,
    specification_lower: f64,
    specification_upper: f64,
}

impl Scenario {
    fn fixtures() -> Vec<Self> {
        vec![
            Self {
                id: "linear_control",
                distribution: InputDistribution::gaussian(1.0, 0.2),
                transform: Transform::Linear { scale: 2.0, offset: 1.0 },
                specification_lower: 2.0,
                specification_upper: 4.0,
            },
            Self {
                id: "quadratic_curvature",
                distribution: InputDistribution::gaussian(1.0, 0.2),
                transform: Transform::Square,
                specification_lower: 0.2,
                specification_upper: 1.8,
            },
            Self {
                id: "exponential_lognormal",
                distribution: InputDistribution::gaussian(0.0, 0.5),
                transform: Transform::Exponential,
                specification_lower: 0.4,
                specification_upper: 2.2,
            },
            Self {
                id: "reciprocal_domain",
                distribution: InputDistribution::gaussian(0.8, 0.2),
                transform: Transform::ReciprocalOneMinusX,
                specification_lower: 2.0,
                specification_upper: 10.0,
            },
            Self {
                id: "bimodal_exponential",
                distribution: InputDistribution::bimodal(),
                transform: Transform::Exponential,
                specification_lower: 0.5,
                specification_upper: 1.5,
            },
        ]
    }
}

#[derive(Clone, Copy, Debug)]
struct Approximation {
    mean: f64,
    standard_uncertainty: f64,
    probability_in_specification: f64,
    decision: Decision,
    available: bool,
}

fn first_order_approximation(
    distribution: &InputDistribution,
    transform: Transform,
    specification_lower: f64,
    specification_upper: f64,
) -> Result<Approximation, String> {
    let (input_mean, input_variance) = distribution.mean_variance()?;
    let Some(mean) = transform.value(input_mean) else {
        return Ok(Approximation {
            mean: f64::NAN, standard_uncertainty: f64::NAN,
            probability_in_specification: f64::NAN, decision: Decision::InvalidDomain,
            available: false,
        });
    };
    let Some(derivative) = transform.first_derivative(input_mean) else {
        return Ok(Approximation {
            mean, standard_uncertainty: f64::NAN,
            probability_in_specification: f64::NAN, decision: Decision::InvalidDomain,
            available: false,
        });
    };
    let standard_uncertainty = derivative.abs() * input_variance.sqrt();
    let probability = normal_interval_probability(
        mean, standard_uncertainty, specification_lower, specification_upper
    );
    Ok(Approximation {
        mean,
        standard_uncertainty,
        probability_in_specification: probability,
        decision: classify_probability(probability, probability),
        available: probability.is_finite(),
    })
}

fn second_order_normal_approximation(
    distribution: &InputDistribution,
    transform: Transform,
    specification_lower: f64,
    specification_upper: f64,
) -> Result<Option<Approximation>, String> {
    if distribution.components.len() != 1 {
        return Ok(None);
    }
    let c = distribution.components[0];
    let Some(f) = transform.value(c.mean) else { return Ok(None); };
    let Some(d1) = transform.first_derivative(c.mean) else { return Ok(None); };
    let Some(d2) = transform.second_derivative(c.mean) else { return Ok(None); };
    let corrected_mean = f + 0.5 * d2 * c.sigma * c.sigma;
    let variance = d1 * d1 * c.sigma * c.sigma
        + 0.5 * d2 * d2 * c.sigma.powi(4);
    if !corrected_mean.is_finite() || !variance.is_finite() || variance <= 0.0 {
        return Ok(None);
    }
    let standard_uncertainty = variance.sqrt();
    let probability = normal_interval_probability(
        corrected_mean, standard_uncertainty, specification_lower, specification_upper
    );
    Ok(Some(Approximation {
        mean: corrected_mean,
        standard_uncertainty,
        probability_in_specification: probability,
        decision: classify_probability(probability, probability),
        available: probability.is_finite(),
    }))
}

fn propagate(
    scenario: &Scenario,
    resolution: usize,
) -> Result<Propagated, String> {
    let (inputs, tail_mass) = scenario.distribution.sample_midpoint_quantiles(resolution)?;
    let mut valid = Vec::with_capacity(inputs.len());
    let mut invalid_domain_mass = 0.0;
    for input in inputs {
        if let Some(value) = scenario.transform.value(input.value) {
            valid.push(WeightedValue { value, weight: input.weight });
        } else {
            invalid_domain_mass += input.weight;
        }
    }
    let valid_mass = valid.iter().map(|v| v.weight).sum::<f64>();
    let unresolved_mass = (tail_mass + invalid_domain_mass).min(1.0);
    if (valid_mass + unresolved_mass - 1.0).abs() > MASS_TOLERANCE {
        return Err(format!(
            "MASS_CONSERVATION_FAILED: valid={valid_mass:.16e}, unresolved={unresolved_mass:.16e}"
        ));
    }

    let mut inside_mass = 0.0;
    for v in &valid {
        if v.value >= scenario.specification_lower && v.value <= scenario.specification_upper {
            inside_mass += v.weight;
        }
    }
    let probability_lower = inside_mass;
    let probability_upper = (inside_mass + unresolved_mass).min(1.0);
    let decision = classify_probability(probability_lower, probability_upper);

    let (
        mean_conditional_valid,
        variance_conditional_valid,
        q025, q05, median, q95, q975,
        histogram_masses,
    ) = if valid_mass > 0.0 {
        valid.sort_by(|a, b| a.value.total_cmp(&b.value));
        let mean = valid.iter().map(|v| v.value * v.weight).sum::<f64>() / valid_mass;
        let variance = valid.iter().map(|v| {
            v.weight * (v.value - mean).powi(2)
        }).sum::<f64>() / valid_mass;
        let quantile = |p: f64| weighted_quantile(&valid, p);
        let min = valid.first().map(|v| v.value).unwrap_or(0.0);
        let max = valid.last().map(|v| v.value).unwrap_or(0.0);
        let mut bins = vec![0.0; 32];
        if max > min {
            for v in &valid {
                let raw = ((v.value - min) / (max - min) * bins.len() as f64).floor() as usize;
                let index = raw.min(bins.len() - 1);
                bins[index] += v.weight;
            }
        } else {
            bins[0] = valid_mass;
        }
        (
            Some(mean), Some(variance),
            quantile(0.025), quantile(0.05), quantile(0.5),
            quantile(0.95), quantile(0.975), bins,
        )
    } else {
        (None, None, None, None, None, None, None, vec![0.0; 32])
    };

    let hist_mass = histogram_masses.iter().sum::<f64>();
    if (hist_mass - valid_mass).abs() > MASS_TOLERANCE {
        return Err("HISTOGRAM_MASS_DOES_NOT_MATCH_VALID_MASS".into());
    }

    Ok(Propagated {
        resolution,
        valid_mass,
        invalid_domain_mass,
        tail_mass,
        unresolved_mass,
        mean_conditional_valid,
        variance_conditional_valid,
        q025_conditional_valid: q025,
        q05_conditional_valid: q05,
        median_conditional_valid: median,
        q95_conditional_valid: q95,
        q975_conditional_valid: q975,
        specification_probability_lower: probability_lower,
        specification_probability_upper: probability_upper,
        decision,
        histogram_masses,
    })
}

fn weighted_quantile(values: &[WeightedValue], probability: f64) -> Option<f64> {
    if values.is_empty() || !(0.0..=1.0).contains(&probability) {
        return None;
    }
    let total = values.iter().map(|v| v.weight).sum::<f64>();
    let target = probability * total;
    let mut cumulative = 0.0;
    for v in values {
        cumulative += v.weight;
        if cumulative >= target {
            return Some(v.value);
        }
    }
    values.last().map(|v| v.value)
}

// Peter Acklam's rational inverse-normal approximation.
fn normal_quantile(p: f64) -> Result<f64, String> {
    if !p.is_finite() || p <= 0.0 || p >= 1.0 {
        return Err("NORMAL_QUANTILE_PROBABILITY_INVALID".into());
    }
    const A: [f64; 6] = [
        -3.969683028665376e1, 2.209460984245205e2,
        -2.759285104469687e2, 1.383577518672690e2,
        -3.066479806614716e1, 2.506628277459239,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e1, 1.615858368580409e2,
        -1.556989798598866e2, 6.680131188771972e1,
        -1.328068155288572e1,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-3, -3.223964580411365e-1,
        -2.400758277161838, -2.549732539343734,
        4.374664141464968, 2.938163982698783,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-3, 3.224671290700398e-1,
        2.445134137142996, 3.754408661907416,
    ];
    let low = 0.02425;
    let high = 1.0 - low;
    let result = if p < low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= high {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    };
    if result.is_finite() { Ok(result) } else { Err("NORMAL_QUANTILE_NONFINITE".into()) }
}

// Abramowitz-Stegun 7.1.26, adequate for the approximate-policy comparator.
fn erf_approx(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let y = 1.0 - (((((1.061405429 * t - 1.453152027) * t + 1.421413741)
        * t - 0.284496736) * t + 0.254829592) * t * (-x * x).exp());
    sign * y
}

fn normal_cdf(z: f64) -> f64 {
    0.5 * (1.0 + erf_approx(z / std::f64::consts::SQRT_2))
}

fn normal_interval_probability(mean: f64, sigma: f64, lower: f64, upper: f64) -> f64 {
    if !mean.is_finite() || !sigma.is_finite() || sigma <= 0.0
        || !lower.is_finite() || !upper.is_finite() || lower > upper {
        return f64::NAN;
    }
    (normal_cdf((upper - mean) / sigma) - normal_cdf((lower - mean) / sigma))
        .clamp(0.0, 1.0)
}

fn analytic_log_normal_moments(mu: f64, sigma: f64) -> (f64, f64) {
    let mean = (mu + 0.5 * sigma * sigma).exp();
    let variance = ((sigma * sigma).exp() - 1.0)
        * (2.0 * mu + sigma * sigma).exp();
    (mean, variance)
}

fn analytic_square_normal_moments(mu: f64, sigma: f64) -> (f64, f64) {
    let mean = mu * mu + sigma * sigma;
    let variance = 4.0 * mu * mu * sigma * sigma + 2.0 * sigma.powi(4);
    (mean, variance)
}

fn correlated_product_moments(
    mean_x: f64,
    sigma_x: f64,
    mean_y: f64,
    sigma_y: f64,
    covariance: f64,
) -> Result<(f64, f64), String> {
    if [mean_x, sigma_x, mean_y, sigma_y, covariance].iter()
        .any(|v| !v.is_finite()) || sigma_x <= 0.0 || sigma_y <= 0.0 {
        return Err("BIVARIATE_INPUT_INVALID".into());
    }
    let determinant = sigma_x.powi(2) * sigma_y.powi(2) - covariance.powi(2);
    if determinant < -1.0e-12 {
        return Err("COVARIANCE_NOT_POSITIVE_SEMIDEFINITE".into());
    }
    let mean = mean_x * mean_y + covariance;
    let variance = mean_x.powi(2) * sigma_y.powi(2)
        + mean_y.powi(2) * sigma_x.powi(2)
        + 2.0 * mean_x * mean_y * covariance
        + sigma_x.powi(2) * sigma_y.powi(2)
        + covariance.powi(2);
    if !mean.is_finite() || !variance.is_finite() || variance < 0.0 {
        return Err("BIVARIATE_MOMENT_INVALID".into());
    }
    Ok((mean, variance))
}

fn scenario_approximations(scenario: &Scenario) -> Result<(Approximation, Option<Approximation>), String> {
    let first = first_order_approximation(
        &scenario.distribution, scenario.transform,
        scenario.specification_lower, scenario.specification_upper
    )?;
    let second = second_order_normal_approximation(
        &scenario.distribution, scenario.transform,
        scenario.specification_lower, scenario.specification_upper
    )?;
    Ok((first, second))
}

fn verify() -> Result<(), String> {
    // Linear control: first-order and propagated moments/probability must agree.
    let linear = &Scenario::fixtures()[0];
    let linear_256 = propagate(linear, 4096)?;
    let (linear_first, _) = scenario_approximations(linear)?;
    let linear_mean = linear_256.mean_conditional_valid.ok_or("LINEAR_MEAN_MISSING")?;
    let linear_var = linear_256.variance_conditional_valid.ok_or("LINEAR_VARIANCE_MISSING")?;
    if (linear_mean - 3.0).abs() > 2.0e-8
        || (linear_var.sqrt() - 0.4).abs() > 2.0e-7
        || (linear_256.specification_probability_lower - linear_first.probability_in_specification).abs() > 2.0e-5 {
        return Err("LINEAR_CONTROL_FAILED".into());
    }

    // Exact normal-square moments and second-order expansion.
    let square = &Scenario::fixtures()[1];
    let square_run = propagate(square, 16384)?;
    let expected_square = analytic_square_normal_moments(1.0, 0.2);
    let square_mean = square_run.mean_conditional_valid.ok_or("SQUARE_MEAN_MISSING")?;
    let square_var = square_run.variance_conditional_valid.ok_or("SQUARE_VARIANCE_MISSING")?;
    if (expected_square.0 - 1.04).abs() > 1.0e-14
        || (expected_square.1 - 0.1632).abs() > 1.0e-14
        || (square_mean - expected_square.0).abs() > 3.0e-5
        || (square_var - expected_square.1).abs() > 2.0e-4 {
        return Err("SQUARE_ANALYTIC_MOMENTS_FAILED".into());
    }
    let (_, square_second) = scenario_approximations(square)?;
    let square_second = square_second.ok_or("SQUARE_SECOND_ORDER_NOT_AVAILABLE")?;
    if (square_second.mean - expected_square.0).abs() > 1.0e-14
        || (square_second.standard_uncertainty.powi(2) - expected_square.1).abs() > 1.0e-14 {
        return Err("SQUARE_SECOND_ORDER_NOT_EXACT".into());
    }

    // Exponential transform has a lognormal analytic oracle.
    let exp = &Scenario::fixtures()[2];
    let exp_run = propagate(exp, 16384)?;
    let expected_exp = analytic_log_normal_moments(0.0, 0.5);
    let exp_mean = exp_run.mean_conditional_valid.ok_or("EXP_MEAN_MISSING")?;
    let exp_var = exp_run.variance_conditional_valid.ok_or("EXP_VARIANCE_MISSING")?;
    if (exp_mean - expected_exp.0).abs() > 2.5e-5
        || (exp_var - expected_exp.1).abs() > 4.0e-4 {
        return Err("EXP_ANALYTIC_MOMENTS_FAILED".into());
    }
    let z975 = 1.959_963_984_540_054;
    let expected_q025 = (-z975 * 0.5).exp();
    let expected_q975 = (z975 * 0.5).exp();
    if (exp_run.q025_conditional_valid.ok_or("EXP_Q025_MISSING")? - expected_q025).abs() > 2.0e-4
        || (exp_run.q975_conditional_valid.ok_or("EXP_Q975_MISSING")? - expected_q975).abs() > 2.0e-4 {
        return Err("EXP_QUANTILE_ORACLE_FAILED".into());
    }

    // Reciprocal transform must expose, not renormalize away, invalid-domain mass.
    let reciprocal = &Scenario::fixtures()[3];
    let reciprocal_run = propagate(reciprocal, 16384)?;
    if reciprocal_run.invalid_domain_mass < 0.15
        || reciprocal_run.invalid_domain_mass > 0.17
        || (reciprocal_run.valid_mass + reciprocal_run.unresolved_mass - 1.0).abs() > MASS_TOLERANCE
        || reciprocal_run.decision != Decision::Indeterminate {
        return Err("RECIPROCAL_DOMAIN_MASS_NOT_PRESERVED".into());
    }

    // A two-mode input pushed through exp remains decision-relevant. A local
    // Gaussian approximation based only on the first two input moments hides
    // that it has almost no probability in the intervening specification band.
    let mixture = &Scenario::fixtures()[4];
    let mixture_run = propagate(mixture, 16384)?;
    let (mixture_first, _) = scenario_approximations(mixture)?;
    if mixture_run.specification_probability_upper > 0.02
        || mixture_run.decision != Decision::Reject
        || mixture_first.decision != Decision::Indeterminate
        || mixture_run.histogram_masses.len() != 32 {
        return Err("MIXTURE_DECISION_DIVERGENCE_NOT_EXPOSED".into());
    }

    // Refinement should narrow/change the statistic estimates toward the fine run.
    let exp_256 = propagate(exp, 256)?;
    let exp_1024 = propagate(exp, 1024)?;
    let exp_4096 = propagate(exp, 4096)?;
    let exp_16384 = propagate(exp, 16384)?;
    let error_256 = (exp_256.mean_conditional_valid.unwrap() - expected_exp.0).abs();
    let error_1024 = (exp_1024.mean_conditional_valid.unwrap() - expected_exp.0).abs();
    let error_4096 = (exp_4096.mean_conditional_valid.unwrap() - expected_exp.0).abs();
    let error_16384 = (exp_16384.mean_conditional_valid.unwrap() - expected_exp.0).abs();
    if error_16384 > 3.0e-5 || error_4096 > 1.0e-4 || error_1024 > 3.0e-4 || error_256 > 1.2e-3 {
        return Err("QUADRATURE_REFINEMENT_ENVELOPE_FAILED".into());
    }

    // Correlated two-input product analytical moments and PSD negative control.
    let product = correlated_product_moments(1.0, 0.2, 2.0, 0.3, 0.03)?;
    let expected_mean = 2.03;
    let expected_variance = 1.0_f64.powi(2) * 0.3_f64.powi(2)
        + 2.0_f64.powi(2) * 0.2_f64.powi(2)
        + 2.0 * 1.0 * 2.0 * 0.03
        + 0.2_f64.powi(2) * 0.3_f64.powi(2)
        + 0.03_f64.powi(2);
    if (product.0 - expected_mean).abs() > 1.0e-14
        || (product.1 - expected_variance).abs() > 1.0e-14
        || correlated_product_moments(0.0, 1.0, 0.0, 1.0, 1.1).is_ok() {
        return Err("CORRELATED_PRODUCT_ANALYTIC_CONTROL_FAILED".into());
    }

    // Every case at every resolution must conserve represented plus unresolved mass.
    for scenario in Scenario::fixtures() {
        scenario.distribution.validate()?;
        for n in RESOLUTIONS {
            let result = propagate(&scenario, n)?;
            if (result.valid_mass + result.unresolved_mass - 1.0).abs() > MASS_TOLERANCE
                || result.specification_probability_lower > result.valid_mass + MASS_TOLERANCE
                || result.specification_probability_upper + MASS_TOLERANCE
                    < result.specification_probability_lower {
                return Err("GLOBAL_MASS_OR_REPORT_VALIDATION_FAILED".into());
            }
        }
    }
    Ok(())
}

fn fmt(value: Option<f64>) -> String {
    value.map(|v| format!("{v:.12e}")).unwrap_or_default()
}

fn emit_csv() -> Result<(), String> {
    println!("scenario_id,transform,resolution,valid_mass,invalid_domain_mass,tail_mass,unresolved_mass,propagated_mean_conditional_valid,propagated_sd_conditional_valid,q025,q05,median,q95,q975,probability_in_spec_lower,probability_in_spec_upper,propagated_decision,first_order_mean,first_order_sd,first_order_probability,first_order_decision,second_order_mean,second_order_sd,second_order_probability,second_order_decision,decision_divergence,histogram_masses");
    for scenario in Scenario::fixtures() {
        let (first, second) = scenario_approximations(&scenario)?;
        for n in RESOLUTIONS {
            let p = propagate(&scenario, n)?;
            let divergence = if first.available && first.decision != p.decision { "true" } else { "false" };
            let second_values = second.map(|s| (
                fmt(Some(s.mean)), fmt(Some(s.standard_uncertainty)),
                fmt(Some(s.probability_in_specification)), s.decision.code().to_string()
            )).unwrap_or_else(|| (String::new(), String::new(), String::new(), "UNAVAILABLE".into()));
            let hist = p.histogram_masses.iter().map(|x| format!("{x:.8e}")).collect::<Vec<_>>().join(";");
            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\"",
                scenario.id, scenario.transform.id(), n,
                fmt(Some(p.valid_mass)), fmt(Some(p.invalid_domain_mass)), fmt(Some(p.tail_mass)),
                fmt(Some(p.unresolved_mass)), fmt(p.mean_conditional_valid),
                fmt(p.variance_conditional_valid.map(f64::sqrt)),
                fmt(p.q025_conditional_valid), fmt(p.q05_conditional_valid),
                fmt(p.median_conditional_valid), fmt(p.q95_conditional_valid),
                fmt(p.q975_conditional_valid), fmt(Some(p.specification_probability_lower)),
                fmt(Some(p.specification_probability_upper)), p.decision.code(),
                fmt(Some(first.mean)), fmt(Some(first.standard_uncertainty)),
                fmt(Some(first.probability_in_specification)), first.decision.code(),
                second_values.0, second_values.1, second_values.2, second_values.3,
                divergence, hist.replace('"', "\"\"")
            );
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("NONLINEAR_MFG_UQ_OK: linearization vs distribution propagation")),
        [arg] if arg == "--verify" => verify().map(|()| println!("NONLINEAR_MFG_UQ_OK: linearization vs distribution propagation")),
        [arg] if arg == "--csv" => emit_csv(),
        [arg] if arg == "--help" || arg == "-h" => {
            println!("Usage: nonlinear-uq [--verify|--csv|--help]");
            Ok(())
        }
        _ => Err("expected --verify, --csv or --help".into()),
    };
    if let Err(error) = result {
        eprintln!("NONLINEAR_MFG_UQ_ERROR: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_normal_known_answers() {
        assert!(normal_quantile(0.5).unwrap().abs() < 1.0e-14);
        assert!((normal_quantile(0.975).unwrap() - 1.959_963_984_540_054).abs() < 1.0e-8);
        assert!((normal_quantile(0.025).unwrap() + 1.959_963_984_540_054).abs() < 1.0e-8);
    }

    #[test]
    fn square_and_log_normal_analytic_moments() {
        let square = analytic_square_normal_moments(1.0, 0.2);
        assert!((square.0 - 1.04).abs() < 1.0e-14);
        assert!((square.1 - 0.1632).abs() < 1.0e-14);
        let lognormal = analytic_log_normal_moments(0.0, 0.5);
        assert!((lognormal.0 - 1.1331484530668263).abs() < 1.0e-14);
        assert!((lognormal.1 - 0.3646958540123865).abs() < 1.0e-14);
    }

    #[test]
    fn deterministic_quadrature_conserves_probability_mass() {
        for scenario in Scenario::fixtures() {
            for n in RESOLUTIONS {
                let summary = propagate(&scenario, n).unwrap();
                assert!((summary.valid_mass + summary.unresolved_mass - 1.0).abs() < MASS_TOLERANCE);
                assert!((summary.histogram_masses.iter().sum::<f64>() - summary.valid_mass).abs() < MASS_TOLERANCE);
                assert!(summary.specification_probability_lower <= summary.specification_probability_upper);
            }
        }
    }

    #[test]
    fn linear_control_has_no_material_decision_or_moment_error() {
        let scenario = &Scenario::fixtures()[0];
        let p = propagate(scenario, 16384).unwrap();
        let (first, _) = scenario_approximations(scenario).unwrap();
        assert!((p.mean_conditional_valid.unwrap() - first.mean).abs() < 2.0e-8);
        assert!((p.variance_conditional_valid.unwrap().sqrt() - first.standard_uncertainty).abs() < 2.0e-7);
        assert_eq!(p.decision, first.decision);
    }

    #[test]
    fn quadratic_second_order_taylor_recovers_exact_normal_moments() {
        let scenario = &Scenario::fixtures()[1];
        let (_, second) = scenario_approximations(scenario).unwrap();
        let second = second.unwrap();
        let exact = analytic_square_normal_moments(1.0, 0.2);
        assert!((second.mean - exact.0).abs() < 1.0e-14);
        assert!((second.standard_uncertainty.powi(2) - exact.1).abs() < 1.0e-14);
    }

    #[test]
    fn nonlinear_mixture_can_reverse_the_conformity_disposition() {
        let scenario = &Scenario::fixtures()[4];
        let p = propagate(scenario, 16384).unwrap();
        let (first, _) = scenario_approximations(scenario).unwrap();
        assert_eq!(p.decision, Decision::Reject);
        assert_eq!(first.decision, Decision::Indeterminate);
        assert!(p.specification_probability_upper < 0.02);
    }

    #[test]
    fn reciprocal_domain_probability_is_not_silently_renormalized() {
        let scenario = &Scenario::fixtures()[3];
        let p = propagate(scenario, 16384).unwrap();
        assert!(p.invalid_domain_mass > 0.15 && p.invalid_domain_mass < 0.17);
        assert!((p.valid_mass + p.unresolved_mass - 1.0).abs() < MASS_TOLERANCE);
        assert_eq!(p.decision, Decision::Indeterminate);
    }

    #[test]
    fn correlated_product_exact_moments_and_psd_validation() {
        let result = correlated_product_moments(1.0, 0.2, 2.0, 0.3, 0.03).unwrap();
        assert!((result.0 - 2.03).abs() < 1.0e-14);
        assert!(correlated_product_moments(0.0, 1.0, 0.0, 1.0, 1.1).is_err());
    }

    #[test]
    fn quadrature_refinement_converges_to_log_normal_moment() {
        let scenario = &Scenario::fixtures()[2];
        let exact = analytic_log_normal_moments(0.0, 0.5).0;
        let errors: Vec<f64> = RESOLUTIONS.iter().map(|n| {
            (propagate(scenario, *n).unwrap().mean_conditional_valid.unwrap() - exact).abs()
        }).collect();
        assert!(errors[0] < 1.0e-3);
        assert!(errors[1] < 2.0e-4);
        assert!(errors[2] < 5.0e-5);
        assert!(errors[3] < 2.0e-5);
    }

    #[test]
    fn deterministic_run_is_byte_identical() {
        let a = emit_capture().unwrap();
        let b = emit_capture().unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    fn emit_capture() -> Result<String, String> {
        let mut out = String::new();
        for scenario in Scenario::fixtures() {
            let (first, second) = scenario_approximations(&scenario)?;
            for n in RESOLUTIONS {
                let p = propagate(&scenario, n)?;
                out.push_str(&format!(
                    "{}|{}|{}|{:.12e}|{:.12e}|{}|{}|{}\n",
                    scenario.id, scenario.transform.id(), n,
                    p.valid_mass, p.unresolved_mass, p.decision.code(),
                    first.decision.code(), second.map(|s| s.decision.code()).unwrap_or("UNAVAILABLE")
                ));
            }
        }
        Ok(out)
    }

    #[test]
    fn malformed_distributions_and_nonphysical_specifications_reject() {
        let empty = InputDistribution { components: Vec::new() };
        assert!(empty.validate().is_err());
        let bad_mass = InputDistribution {
            components: vec![GaussianComponent { weight: 0.8, mean: 0.0, sigma: 1.0 }]
        };
        assert!(bad_mass.validate().is_err());
        assert!(normal_quantile(0.0).is_err());
        assert!(normal_quantile(1.0).is_err());
        assert!(correlated_product_moments(0.0, 0.0, 0.0, 1.0, 0.0).is_err());
    }
}
