#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestStatisticValues {
    pub laplace: f64,
    pub greenwood: f64,
    pub cramer_von_mises: f64,
    pub anderson_darling: f64,
    pub kolmogorov_smirnov: f64,
}

fn log_one_minus_exp(value: f64) -> Result<f64, String> {
    if !value.is_finite() || value >= 0.0 {
        return Err("log transform must be finite and strictly negative".to_owned());
    }
    let result = if value < -std::f64::consts::LN_2 {
        (-value.exp()).ln_1p()
    } else {
        (-value.exp_m1()).ln()
    };
    if result.is_finite() {
        Ok(result)
    } else {
        Err("stable log(1-exp(x)) evaluation failed".to_owned())
    }
}

/// Calculate the five predeclared PLP GOF statistics from ordered log-transforms.
///
/// Input values are log(U_hat_(j)) in ascending order. Keeping the log scale
/// avoids underflow in Anderson-Darling tail terms for very small fitted times.
pub fn from_sorted_log_transforms(log_values: &[f64]) -> Result<TestStatisticValues, String> {
    let count = log_values.len();
    if count == 0 {
        return Err("GOF statistic input must be non-empty".to_owned());
    }

    let n = count as f64;
    let mut values = Vec::with_capacity(count);
    let mut previous = f64::NEG_INFINITY;
    for value in log_values {
        if !value.is_finite() || *value >= 0.0 || *value < previous {
            return Err("log transforms must be finite, strictly negative and ordered".to_owned());
        }
        previous = *value;
        values.push(value.exp());
    }

    let mut laplace_sum = 0.0;
    let mut cramer_von_mises = 1.0 / (12.0 * n);
    let mut anderson_darling_sum = 0.0;
    let mut d_plus: f64 = 0.0;
    let mut d_minus: f64 = 0.0;
    let mut greenwood = 0.0;
    let mut previous_value = 0.0;

    for (index, value) in values.iter().enumerate() {
        let rank = index + 1;
        laplace_sum += *value - 0.5;
        let expected_midpoint = (2 * rank - 1) as f64 / (2.0 * n);
        cramer_von_mises += (*value - expected_midpoint).powi(2);

        let upper_empirical = rank as f64 / n;
        let lower_empirical = (rank - 1) as f64 / n;
        d_plus = d_plus.max(upper_empirical - value);
        d_minus = d_minus.max(value - lower_empirical);

        let reverse_log = log_values[count - rank];
        anderson_darling_sum += (2 * rank - 1) as f64
            * (*log_values.get(index).expect("index is in range")
                + log_one_minus_exp(reverse_log)?);

        let gap = *value - previous_value;
        greenwood += gap * gap;
        previous_value = *value;
    }

    let final_gap = 1.0 - previous_value;
    greenwood += final_gap * final_gap;

    let result = TestStatisticValues {
        laplace: (12.0 / n).sqrt() * laplace_sum,
        greenwood,
        cramer_von_mises,
        anderson_darling: -anderson_darling_sum / n - n,
        kolmogorov_smirnov: d_plus.max(d_minus),
    };

    if !result.laplace.is_finite() {
        return Err("Laplace statistic is non-finite".to_owned());
    }
    if result.greenwood <= 0.0
        || result.cramer_von_mises < 0.0
        || result.anderson_darling < -1.0e-10
        || result.kolmogorov_smirnov < 0.0
    {
        return Err("one or more GOF statistics are outside their valid range".to_owned());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_have_known_values_for_simple_ordered_sample() {
        let values = [0.1_f64, 0.5, 0.9];
        let logs = values.map(f64::ln);
        let stats = from_sorted_log_transforms(&logs).expect("valid log transforms");

        assert!((stats.laplace).abs() < 1.0e-14);
        assert!((stats.greenwood - 0.34).abs() < 1.0e-14);
        assert!((stats.cramer_von_mises - 0.036_666_666_666_666_67).abs() < 1.0e-14);
        assert!((stats.anderson_darling - 0.272_552_808_642_008_15).abs() < 1.0e-12);
        assert!((stats.kolmogorov_smirnov - 0.233_333_333_333_333_4).abs() < 1.0e-14);
    }

    #[test]
    fn tail_weighted_statistic_remains_finite_under_exponential_underflow() {
        let logs = [-1_000.0, -30.0, -2.0, -0.2];
        let stats = from_sorted_log_transforms(&logs).expect("finite log-domain statistics");
        assert!(stats.anderson_darling.is_finite());
        assert!(stats.greenwood.is_finite());
        assert!(stats.kolmogorov_smirnov.is_finite());
    }

    #[test]
    fn log_transforms_must_be_ordered_and_strictly_negative() {
        assert!(from_sorted_log_transforms(&[-0.1, -0.2]).is_err());
        assert!(from_sorted_log_transforms(&[-0.1, 0.0]).is_err());
        assert!(from_sorted_log_transforms(&[]).is_err());
    }
}
