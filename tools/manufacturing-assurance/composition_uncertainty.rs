//! Synthetic known-truth covariance propagation across composed twins.
//!
//! Demonstrates why component marginal variances cannot be composed under an
//! independence assumption when subsystem estimates share latent uncertainty.
//! This is a small linear 2x2 reference, not a complete GUM implementation.
//!
//! Build and test:
//!   rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/composition_uncertainty.rs -o /tmp/mfg-uq-tests
//!   /tmp/mfg-uq-tests
//!
//! Build and run:
//!   rustc --edition=2021 -D warnings tools/manufacturing-assurance/composition_uncertainty.rs -o /tmp/mfg-uq
//!   /tmp/mfg-uq

use std::collections::BTreeMap;

const COVARIANCE_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Covariance2 {
    xx: f64,
    xy: f64,
    yx: f64,
    yy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LinearMap2 {
    a11: f64,
    a12: f64,
    a21: f64,
    a22: f64,
}

impl LinearMap2 {
    fn validate(self) -> Result<(), String> {
        if [self.a11, self.a12, self.a21, self.a22]
            .iter()
            .all(|value| value.is_finite())
        {
            Ok(())
        } else {
            Err("linear map contains a non-finite coefficient".to_string())
        }
    }

    fn multiply(self, right: Self) -> Result<Self, String> {
        self.validate()?;
        right.validate()?;
        let result = Self {
            a11: self.a11 * right.a11 + self.a12 * right.a21,
            a12: self.a11 * right.a12 + self.a12 * right.a22,
            a21: self.a21 * right.a11 + self.a22 * right.a21,
            a22: self.a21 * right.a12 + self.a22 * right.a22,
        };
        result.validate()?;
        Ok(result)
    }
}

impl Covariance2 {
    fn validate(self) -> Result<(), String> {
        let entries = [self.xx, self.xy, self.yx, self.yy];
        if !entries.iter().all(|value| value.is_finite()) {
            return Err("covariance contains a non-finite value".to_string());
        }
        if self.xx < 0.0 || self.yy < 0.0 {
            return Err("covariance has a negative variance".to_string());
        }

        let scale = self
            .xx
            .abs()
            .max(self.yy.abs())
            .max(self.xy.abs())
            .max(self.yx.abs())
            .max(1.0);
        let tolerance = COVARIANCE_TOLERANCE * scale;
        if (self.xy - self.yx).abs() > tolerance {
            return Err("covariance matrix is not symmetric".to_string());
        }

        let determinant = self.xx * self.yy - self.xy * self.yx;
        let determinant_scale = (self.xx * self.yy)
            .abs()
            .max((self.xy * self.yx).abs())
            .max(1.0);
        if !determinant.is_finite() {
            return Err("covariance determinant is non-finite".to_string());
        }
        if determinant < -COVARIANCE_TOLERANCE * determinant_scale {
            return Err("covariance matrix is not positive semidefinite".to_string());
        }
        Ok(())
    }

    fn propagate(self, map: LinearMap2) -> Result<Self, String> {
        self.validate()?;
        map.validate()?;

        // U_y = A U_x A^T, written out to keep the dependency-free
        // reference transparent and straightforward to audit.
        let au11 = map.a11 * self.xx + map.a12 * self.yx;
        let au12 = map.a11 * self.xy + map.a12 * self.yy;
        let au21 = map.a21 * self.xx + map.a22 * self.yx;
        let au22 = map.a21 * self.xy + map.a22 * self.yy;

        let result = Self {
            xx: au11 * map.a11 + au12 * map.a12,
            xy: au11 * map.a21 + au12 * map.a22,
            yx: au21 * map.a11 + au22 * map.a12,
            yy: au21 * map.a21 + au22 * map.a22,
        };
        result.validate()?;
        Ok(result)
    }

    fn correlation(self) -> Result<f64, String> {
        self.validate()?;
        let denominator = (self.xx * self.yy).sqrt();
        if denominator == 0.0 {
            if self.xy == 0.0 && self.yx == 0.0 {
                return Ok(0.0);
            }
            return Err("correlation undefined for zero variance".to_string());
        }
        let result = (self.xy + self.yx) / (2.0 * denominator);
        if !result.is_finite() || result.abs() > 1.0 + COVARIANCE_TOLERANCE {
            return Err("correlation is outside the admissible range".to_string());
        }
        Ok(result.clamp(-1.0, 1.0))
    }
}

fn shared_latent_covariance() -> Covariance2 {
    // X_A = common_bias + local_error_A; X_B = common_bias + local_error_B.
    // Var(common_bias)=4, Var(local_A)=1, Var(local_B)=2.25.
    Covariance2 {
        xx: 5.0,
        xy: 4.0,
        yx: 4.0,
        yy: 6.25,
    }
}

fn sum_difference_map() -> LinearMap2 {
    LinearMap2 {
        a11: 1.0,
        a12: 1.0,
        a21: 1.0,
        a22: -1.0,
    }
}

fn independent_false_control() -> Covariance2 {
    let covariance = shared_latent_covariance();
    Covariance2 {
        xy: 0.0,
        yx: 0.0,
        ..covariance
    }
}

fn fixture_values() -> Result<BTreeMap<String, f64>, String> {
    let fixture = include_str!("fixtures/shared-latent-covariance.tsv");
    let mut values = BTreeMap::new();
    for (index, line) in fixture.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        if fields.len() != 2 || fields[0].is_empty() {
            return Err(format!("fixture line {} has invalid shape", index + 1));
        }
        let value = fields[1]
            .parse::<f64>()
            .map_err(|_| format!("fixture line {} has invalid number", index + 1))?;
        if !value.is_finite() {
            return Err(format!("fixture line {} is non-finite", index + 1));
        }
        if values.insert(fields[0].to_string(), value).is_some() {
            return Err(format!("fixture line {} duplicates a key", index + 1));
        }
    }
    Ok(values)
}

fn required_fixture_value(values: &BTreeMap<String, f64>, key: &str) -> Result<f64, String> {
    values
        .get(key)
        .copied()
        .ok_or_else(|| format!("fixture is missing {key}"))
}

fn run_report() -> Result<String, String> {
    let input = shared_latent_covariance();
    let correlated = input.propagate(sum_difference_map())?;
    let independent = independent_false_control().propagate(sum_difference_map())?;
    let correlation = input.correlation()?;
    let independence_sum_understatement_pct =
        (correlated.xx - independent.xx) / correlated.xx * 100.0;
    let sum_standard_uncertainty = correlated.xx.sqrt();
    let difference_standard_uncertainty = correlated.yy.sqrt();

    let mut output = String::from(
        "MFG_COMPOSITION_UQ version=1 method=linear_covariance_propagation\n",
    );
    output.push_str(&format!(
        "INPUT var_a_k2={:.6} cov_ab_k2={:.6} var_b_k2={:.6} correlation={:.6}\n",
        input.xx, input.xy, input.yy, correlation
    ));
    output.push_str(&format!(
        "OUTPUT model=shared_latent sum_variance_k2={:.6} difference_variance_k2={:.6} sum_difference_covariance_k2={:.6} sum_std_uncertainty_k={:.6} difference_std_uncertainty_k={:.6}\n",
        correlated.xx,
        correlated.yy,
        correlated.xy,
        sum_standard_uncertainty,
        difference_standard_uncertainty
    ));
    output.push_str(&format!(
        "OUTPUT model=incorrect_independence sum_variance_k2={:.6} difference_variance_k2={:.6}\n",
        independent.xx, independent.yy
    ));
    output.push_str(&format!(
        "INDEPENDENCE_BIAS sum_variance_understatement_pct={:.6}\n",
        independence_sum_understatement_pct
    ));
    output.push_str(
        "CLAIM_CEILING linear_analytic_covariance=true full_gum_implementation=false monte_carlo_validation=false production_uq=false decision_authority=false\n",
    );
    Ok(output)
}

#[cfg(not(test))]
fn main() {
    match run_report() {
        Ok(report) => print!("{report}"),
        Err(error) => {
            eprintln!("mfg-composition-uncertainty: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_latent_fixture_matches_covariance_propagation() {
        let values = fixture_values().unwrap();
        let input = shared_latent_covariance();
        let actual = input.propagate(sum_difference_map()).unwrap();

        assert!((input.xx - required_fixture_value(&values, "var_a_k2").unwrap()).abs() < 1.0e-12);
        assert!((input.xy - required_fixture_value(&values, "cov_ab_k2").unwrap()).abs() < 1.0e-12);
        assert!((input.yy - required_fixture_value(&values, "var_b_k2").unwrap()).abs() < 1.0e-12);
        assert!((actual.xx - required_fixture_value(&values, "variance_sum_k2").unwrap()).abs() < 1.0e-12);
        assert!((actual.yy - required_fixture_value(&values, "variance_difference_k2").unwrap()).abs() < 1.0e-12);
        assert!((actual.xy - required_fixture_value(&values, "covariance_sum_difference_k2").unwrap()).abs() < 1.0e-12);
        assert!((independent_false_control().propagate(sum_difference_map()).unwrap().xx
            - required_fixture_value(&values, "independence_variance_k2").unwrap()).abs() < 1.0e-12);
    }

    #[test]
    fn shared_common_mode_uncertainty_is_not_independent() {
        let input = shared_latent_covariance();
        let correlated = input.propagate(sum_difference_map()).unwrap();
        let independent = independent_false_control().propagate(sum_difference_map()).unwrap();
        assert!(correlated.xx > independent.xx);
        assert!(correlated.yy < independent.yy);
        assert!((input.correlation().unwrap() - 4.0 / (5.0_f64 * 6.25).sqrt()).abs() < 1.0e-12);
    }

    #[test]
    fn negative_correlation_reverses_sum_and_difference_effect() {
        let input = Covariance2 {
            xx: 5.0,
            xy: -4.0,
            yx: -4.0,
            yy: 6.25,
        };
        let output = input.propagate(sum_difference_map()).unwrap();
        assert!((output.xx - 3.25).abs() < 1.0e-12);
        assert!((output.yy - 19.25).abs() < 1.0e-12);
    }

    #[test]
    fn zero_covariance_matches_independent_control() {
        let input = independent_false_control();
        let output = input.propagate(sum_difference_map()).unwrap();
        assert!((output.xx - 11.25).abs() < 1.0e-12);
        assert!((output.yy - 11.25).abs() < 1.0e-12);
        assert!((output.xy + 1.25).abs() < 1.0e-12);
    }

    #[test]
    fn covariance_composition_is_associative_for_linear_maps() {
        let first = LinearMap2 {
            a11: 1.0,
            a12: 1.0,
            a21: 1.0,
            a22: -1.0,
        };
        let second = LinearMap2 {
            a11: 2.0,
            a12: 0.5,
            a21: -1.0,
            a22: 3.0,
        };
        let composed_map = second.multiply(first).unwrap();
        let sequential = shared_latent_covariance()
            .propagate(first)
            .unwrap()
            .propagate(second)
            .unwrap();
        let direct = shared_latent_covariance()
            .propagate(composed_map)
            .unwrap();
        assert!((sequential.xx - direct.xx).abs() < 1.0e-10);
        assert!((sequential.xy - direct.xy).abs() < 1.0e-10);
        assert!((sequential.yx - direct.yx).abs() < 1.0e-10);
        assert!((sequential.yy - direct.yy).abs() < 1.0e-10);
    }

    #[test]
    fn identity_transform_preserves_covariance() {
        let identity = LinearMap2 {
            a11: 1.0,
            a12: 0.0,
            a21: 0.0,
            a22: 1.0,
        };
        assert_eq!(
            shared_latent_covariance().propagate(identity).unwrap(),
            shared_latent_covariance()
        );
    }

    #[test]
    fn invalid_non_positive_semidefinite_covariance_is_rejected() {
        let invalid = Covariance2 {
            xx: 1.0,
            xy: 2.0,
            yx: 2.0,
            yy: 1.0,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn asymmetric_covariance_is_rejected() {
        let invalid = Covariance2 {
            xx: 1.0,
            xy: 0.2,
            yx: 0.1,
            yy: 1.0,
        };
        assert_eq!(
            invalid.validate(),
            Err("covariance matrix is not symmetric".to_string())
        );
    }

    #[test]
    fn negative_variance_is_rejected() {
        let invalid = Covariance2 {
            xx: -0.1,
            xy: 0.0,
            yx: 0.0,
            yy: 1.0,
        };
        assert_eq!(
            invalid.validate(),
            Err("covariance has a negative variance".to_string())
        );
    }

    #[test]
    fn non_finite_covariance_is_rejected() {
        let invalid = Covariance2 {
            xx: 1.0,
            xy: f64::NAN,
            yx: f64::NAN,
            yy: 1.0,
        };
        assert_eq!(
            invalid.validate(),
            Err("covariance contains a non-finite value".to_string())
        );
    }

    #[test]
    fn non_finite_linear_map_is_rejected() {
        let invalid = LinearMap2 {
            a11: f64::INFINITY,
            a12: 0.0,
            a21: 0.0,
            a22: 1.0,
        };
        assert!(shared_latent_covariance().propagate(invalid).is_err());
    }

    #[test]
    fn correlation_is_bounded_and_sign_sensitive() {
        let positive = shared_latent_covariance().correlation().unwrap();
        let negative = Covariance2 {
            xy: -4.0,
            yx: -4.0,
            ..shared_latent_covariance()
        }
        .correlation()
        .unwrap();
        assert!(positive > 0.0 && positive < 1.0);
        assert!(negative < 0.0 && negative > -1.0);
    }

    #[test]
    fn deterministic_report_replays_identically() {
        assert_eq!(run_report().unwrap(), run_report().unwrap());
    }

    #[test]
    fn report_exposes_claim_ceiling() {
        let report = run_report().unwrap();
        assert!(report.contains("full_gum_implementation=false"));
        assert!(report.contains("monte_carlo_validation=false"));
        assert!(report.contains("production_uq=false"));
    }

    #[test]
    fn fixture_rejects_duplicate_or_malformed_keys() {
        assert!(fixture_values().is_ok());
        let mut values = BTreeMap::new();
        assert!(values.insert("var_a_k2".to_string(), 5.0).is_none());
        assert!(values.insert("var_a_k2".to_string(), 6.0).is_some());
    }
}
