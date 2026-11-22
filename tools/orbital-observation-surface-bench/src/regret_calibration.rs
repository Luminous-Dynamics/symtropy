//! Exact finite-sample, per-regime regret calibration rank kernel.
//!
//! This module implements a conservative split-conformal rank baseline for a
//! predeclared finite set of operating regimes. It does not authenticate the score
//! contract, prove score validity, or create an anytime-valid guarantee. The
//! unknown-regime envelope is the maximum of valid per-regime thresholds.

use crate::decision_identifiability::Rational;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::BTreeSet;

const CORPUS_SCHEMA: &str = "luminous.orbital-regret-calibration.corpus.v0.1";
const CLAIM_CEILING: &str = "finite-split-conformal-regret-rank-algebra-only";
const MAX_REGIMES: usize = 64;
const MAX_SAMPLES_PER_REGIME: usize = 100_000;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationSample {
    pub sample_id: String,
    /// Identifies the independently sampled replication unit, e.g. a session/run.
    /// Repeated or shared units must not be counted as independent calibration scores.
    pub replication_unit_id: String,
    /// Domain-owned score contract says this upper-bounds realized decision regret.
    pub regret_upper_bound_score: Rational,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegimeCalibration {
    pub regime_id: String,
    /// Planned inventory, frozen before calibration outcomes are observed.
    pub expected_sample_ids: Vec<String>,
    pub expected_sample_ids_sha256: String,
    /// Digest of the complete sample records, sorted by sample_id.
    pub samples_sha256: String,
    pub samples: Vec<CalibrationSample>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationCorpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub calibration_split_id: String,
    pub score_contract_id: String,
    /// Opaque reference to the independently reviewed upper-bound score contract.
    pub score_contract_receipt_id: String,
    /// The split and regime/score selection must be frozen before outcomes are seen.
    pub selection_status: String,
    /// v0.1 supports fixed-horizon split calibration only.
    pub stopping_rule: String,
    pub alpha: Rational,
    /// Optional independently supported bound for the true score's range.
    pub score_support_upper_bound: Option<Rational>,
    pub score_support_receipt_id: Option<String>,
    pub expected_regime_ids: Vec<String>,
    pub expected_regime_ids_sha256: String,
    pub regimes: Vec<RegimeCalibration>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegimeResult {
    pub regime_id: String,
    pub sample_count: usize,
    /// One-indexed split-conformal order-statistic rank.
    pub rank: usize,
    pub threshold_status: String,
    /// Missing only when rank exceeds the calibration sample and no support bound exists.
    pub threshold: Option<Rational>,
    pub selected_sample_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CalibrationResult {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub calibration_split_id: String,
    pub score_contract_id: String,
    pub alpha: Rational,
    pub regime_count: usize,
    pub total_sample_count: usize,
    pub global_status: String,
    /// Conservative threshold valid for any mixture of the declared regimes if
    /// each regime's own exchangeability assumption holds.
    pub worst_regime_threshold: Option<Rational>,
    pub worst_regime_ids: Vec<String>,
    pub regimes_without_finite_nontrivial_threshold: Vec<String>,
    pub regime_results: Vec<RegimeResult>,
    pub coverage_claim: String,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("failed to serialize canonical calibration IDs: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn canonical_samples(samples: &[CalibrationSample]) -> Result<Value, String> {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|left, right| left.sample_id.cmp(&right.sample_id));
    serde_json::to_value(sorted)
        .map_err(|error| format!("failed to encode canonical calibration samples: {error}"))
}

fn digest_samples(samples: &[CalibrationSample]) -> Result<String, String> {
    let canonical = canonical_samples(samples)?;
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("failed to serialize calibration sample records: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii()
}

fn ceil_div(numerator: u128, denominator: u128) -> u128 {
    numerator / denominator + if numerator % denominator == 0 { 0 } else { 1 }
}

/// Exact split-conformal rank: ceil((n + 1) * (1 - alpha)).
fn conformal_rank(sample_count: usize, alpha: &Rational) -> Result<usize, String> {
    let denominator = u128::from(alpha.denominator);
    let one_minus_numerator = denominator
        .checked_sub(u128::from(alpha.numerator))
        .ok_or_else(|| "alpha must be below one".to_owned())?;
    let multiplier = (sample_count as u128)
        .checked_add(1)
        .ok_or_else(|| "calibration sample count overflow".to_owned())?;
    let numerator = multiplier
        .checked_mul(one_minus_numerator)
        .ok_or_else(|| "conformal rank numerator overflow".to_owned())?;
    usize::try_from(ceil_div(numerator, denominator))
        .map_err(|_| "conformal rank does not fit usize".to_owned())
}

fn validate(corpus: &CalibrationCorpus) -> Result<(), String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || !valid_id(&corpus.calibration_split_id)
        || !valid_id(&corpus.score_contract_id)
        || !valid_id(&corpus.score_contract_receipt_id)
        || corpus.selection_status != "predeclared"
        || corpus.stopping_rule != "fixed"
        || corpus.regimes.is_empty()
        || corpus.regimes.len() > MAX_REGIMES
    {
        return Err("unsupported calibration corpus, score contract, selection or stopping rule".to_owned());
    }

    corpus
        .alpha
        .validate()
        .map_err(|error| format!("alpha is invalid: {error}"))?;
    if corpus.alpha.numerator == 0 || corpus.alpha.numerator >= corpus.alpha.denominator {
        return Err("alpha must be strictly between zero and one".to_owned());
    }

    match (
        corpus.score_support_upper_bound.as_ref(),
        corpus.score_support_receipt_id.as_deref(),
    ) {
        (Some(bound), Some(receipt)) if valid_id(receipt) => {
            bound
                .validate()
                .map_err(|error| format!("score support bound is invalid: {error}"))?;
        }
        (None, None) => {}
        _ => {
            return Err(
                "score support bound and its evidence receipt must be supplied together".to_owned(),
            );
        }
    }

    let mut expected_regimes = BTreeSet::new();
    for regime_id in &corpus.expected_regime_ids {
        if !valid_id(regime_id) || !expected_regimes.insert(regime_id.as_str()) {
            return Err("planned regime IDs must be unique, nonempty ASCII strings".to_owned());
        }
    }
    if expected_regimes.is_empty() || expected_regimes.len() != corpus.regimes.len() {
        return Err("planned regime inventory must be nonempty and match regime count".to_owned());
    }
    if digest_ids(&corpus.expected_regime_ids)? != corpus.expected_regime_ids_sha256 {
        return Err("planned regime inventory digest mismatch".to_owned());
    }

    let mut observed_regimes = BTreeSet::new();
    let mut global_sample_ids = BTreeSet::new();
    let mut global_replication_units = BTreeSet::new();
    for regime in &corpus.regimes {
        if !valid_id(&regime.regime_id)
            || !observed_regimes.insert(regime.regime_id.as_str())
            || regime.samples.is_empty()
            || regime.samples.len() > MAX_SAMPLES_PER_REGIME
            || regime.expected_sample_ids.len() != regime.samples.len()
        {
            return Err("regime identity, sample count or planned inventory is invalid".to_owned());
        }

        let mut expected_samples = BTreeSet::new();
        for sample_id in &regime.expected_sample_ids {
            if !valid_id(sample_id) || !expected_samples.insert(sample_id.as_str()) {
                return Err("planned sample IDs must be unique, nonempty ASCII strings".to_owned());
            }
        }
        if digest_ids(&regime.expected_sample_ids)? != regime.expected_sample_ids_sha256 {
            return Err(format!("planned sample ID digest mismatch for regime {}", regime.regime_id));
        }

        let mut observed_samples = BTreeSet::new();
        let mut observed_units = BTreeSet::new();
        for sample in &regime.samples {
            if !valid_id(&sample.sample_id)
                || !valid_id(&sample.replication_unit_id)
                || !observed_samples.insert(sample.sample_id.as_str())
                || !observed_units.insert(sample.replication_unit_id.as_str())
                || !global_sample_ids.insert(sample.sample_id.as_str())
                || !global_replication_units.insert(sample.replication_unit_id.as_str())
            {
                return Err("calibration sample IDs and independent replication units must be unique".to_owned());
            }
            sample
                .regret_upper_bound_score
                .validate()
                .map_err(|error| format!("regret score is invalid: {error}"))?;
            if let Some(bound) = &corpus.score_support_upper_bound {
                if sample.regret_upper_bound_score.compare_exact(bound) == Ordering::Greater {
                    return Err(format!("score exceeds declared support bound in regime {}", regime.regime_id));
                }
            }
        }

        if observed_samples != expected_samples {
            return Err(format!("observed sample IDs do not match planned inventory for regime {}", regime.regime_id));
        }
        if digest_samples(&regime.samples)? != regime.samples_sha256 {
            return Err(format!("calibration sample digest mismatch for regime {}", regime.regime_id));
        }
    }

    if observed_regimes != expected_regimes {
        return Err("observed regimes do not match planned regime inventory".to_owned());
    }

    Ok(())
}

fn evaluate_regime(
    regime: &RegimeCalibration,
    alpha: &Rational,
    support_bound: Option<&Rational>,
) -> Result<RegimeResult, String> {
    let rank = conformal_rank(regime.samples.len(), alpha)?;
    let mut sorted = regime.samples.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.regret_upper_bound_score
            .compare_exact(&right.regret_upper_bound_score)
            .then_with(|| left.sample_id.cmp(&right.sample_id))
    });

    if rank <= sorted.len() {
        let selected = sorted[rank - 1];
        return Ok(RegimeResult {
            regime_id: regime.regime_id.clone(),
            sample_count: sorted.len(),
            rank,
            threshold_status: "EMPIRICAL_ORDER_STATISTIC".to_owned(),
            threshold: Some(selected.regret_upper_bound_score.canonicalized()),
            selected_sample_id: Some(selected.sample_id.clone()),
        });
    }

    if let Some(bound) = support_bound {
        return Ok(RegimeResult {
            regime_id: regime.regime_id.clone(),
            sample_count: sorted.len(),
            rank,
            threshold_status: "SUPPORT_BOUND_ONLY".to_owned(),
            threshold: Some(bound.canonicalized()),
            selected_sample_id: None,
        });
    }

    Ok(RegimeResult {
        regime_id: regime.regime_id.clone(),
        sample_count: sorted.len(),
        rank,
        threshold_status: "NO_FINITE_NONTRIVIAL_THRESHOLD".to_owned(),
        threshold: None,
        selected_sample_id: None,
    })
}

pub fn evaluate(corpus: &CalibrationCorpus) -> Result<CalibrationResult, String> {
    validate(corpus)?;

    let mut regime_results = corpus
        .regimes
        .iter()
        .map(|regime| {
            evaluate_regime(
                regime,
                &corpus.alpha,
                corpus.score_support_upper_bound.as_ref(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    regime_results.sort_by(|left, right| left.regime_id.cmp(&right.regime_id));

    let regimes_without_finite_nontrivial_threshold = regime_results
        .iter()
        .filter(|item| item.threshold.is_none())
        .map(|item| item.regime_id.clone())
        .collect::<Vec<_>>();

    let (global_status, worst_regime_threshold, worst_regime_ids) =
        if !regimes_without_finite_nontrivial_threshold.is_empty() {
            (
                "NO_FINITE_NONTRIVIAL_THRESHOLD".to_owned(),
                None,
                Vec::new(),
            )
        } else {
            let threshold = regime_results
                .iter()
                .filter_map(|item| item.threshold.as_ref())
                .max_by(|left, right| left.compare_exact(right))
                .ok_or_else(|| "finite regime threshold set unexpectedly empty".to_owned())?
                .canonicalized();
            let worst_regime_ids = regime_results
                .iter()
                .filter(|item| {
                    item.threshold
                        .as_ref()
                        .is_some_and(|value| value.exactly_equals(&threshold))
                })
                .map(|item| item.regime_id.clone())
                .collect::<Vec<_>>();
            let status = if regime_results
                .iter()
                .any(|item| item.threshold_status == "SUPPORT_BOUND_ONLY")
            {
                "WORST_REGIME_SUPPORT_BOUND_ONLY"
            } else {
                "WORST_REGIME_EMPIRICAL_THRESHOLD"
            };
            (status.to_owned(), Some(threshold), worst_regime_ids)
        };

    Ok(CalibrationResult {
        result: "pass".to_owned(),
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        calibration_split_id: corpus.calibration_split_id.clone(),
        score_contract_id: corpus.score_contract_id.clone(),
        alpha: corpus.alpha.canonicalized(),
        regime_count: regime_results.len(),
        total_sample_count: regime_results.iter().map(|item| item.sample_count).sum(),
        global_status,
        worst_regime_threshold,
        worst_regime_ids,
        regimes_without_finite_nontrivial_threshold,
        regime_results,
        coverage_claim: "marginal_per_declared_regime_under_within_regime_exchangeability_only".to_owned(),
        physical_qualification_granted: false,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, unit: &str, numerator: u64) -> CalibrationSample {
        CalibrationSample {
            sample_id: id.to_owned(),
            replication_unit_id: unit.to_owned(),
            regret_upper_bound_score: Rational {
                numerator,
                denominator: 10,
            },
        }
    }

    fn regime(id: &str, samples: Vec<CalibrationSample>) -> RegimeCalibration {
        let ids = samples.iter().map(|item| item.sample_id.clone()).collect::<Vec<_>>();
        RegimeCalibration {
            regime_id: id.to_owned(),
            expected_sample_ids_sha256: digest_ids(&ids).unwrap(),
            expected_sample_ids: ids,
            samples_sha256: digest_samples(&samples).unwrap(),
            samples,
        }
    }

    fn corpus(regimes: Vec<RegimeCalibration>, alpha: Rational, support: Option<Rational>) -> CalibrationCorpus {
        let mut ids = regimes.iter().map(|item| item.regime_id.clone()).collect::<Vec<_>>();
        ids.sort();
        CalibrationCorpus {
            schema: CORPUS_SCHEMA.to_owned(),
            evidence_class: "synthetic".to_owned(),
            claim_ceiling: CLAIM_CEILING.to_owned(),
            calibration_split_id: "split-v0".to_owned(),
            score_contract_id: "regret-upper-bound-v0".to_owned(),
            score_contract_receipt_id: "score-contract-receipt".to_owned(),
            selection_status: "predeclared".to_owned(),
            stopping_rule: "fixed".to_owned(),
            alpha,
            score_support_upper_bound: support,
            score_support_receipt_id: support.as_ref().map(|_| "support-bound-receipt".to_owned()),
            expected_regime_ids_sha256: digest_ids(&ids).unwrap(),
            expected_regime_ids: ids,
            regimes,
        }
    }

    #[test]
    fn exact_order_statistic_uses_one_indexed_conformal_rank() {
        let r = regime("dry", vec![
            sample("s1", "u1", 1),
            sample("s2", "u2", 4),
            sample("s3", "u3", 8),
        ]);
        let input = corpus(vec![r], Rational { numerator: 1, denominator: 4 }, None);
        let result = evaluate(&input).unwrap();
        assert_eq!(result.regime_results[0].rank, 3);
        assert_eq!(result.regime_results[0].threshold_status, "EMPIRICAL_ORDER_STATISTIC");
        assert_eq!(result.regime_results[0].threshold.unwrap().numerator, 8);
    }

    #[test]
    fn no_support_bound_never_uses_largest_observed_score_as_fake_quantile() {
        let r = regime("sparse", vec![
            sample("s1", "u1", 1),
            sample("s2", "u2", 2),
            sample("s3", "u3", 3),
        ]);
        let input = corpus(vec![r], Rational { numerator: 1, denominator: 10 }, None);
        let result = evaluate(&input).unwrap();
        assert_eq!(result.regime_results[0].rank, 4);
        assert_eq!(result.global_status, "NO_FINITE_NONTRIVIAL_THRESHOLD");
        assert!(result.worst_regime_threshold.is_none());
    }

    #[test]
    fn independent_support_bound_is_reported_as_support_only_not_empirical() {
        let r = regime("sparse", vec![
            sample("s1", "u1", 1),
            sample("s2", "u2", 2),
            sample("s3", "u3", 3),
        ]);
        let input = corpus(
            vec![r],
            Rational { numerator: 1, denominator: 10 },
            Some(Rational { numerator: 1, denominator: 1 }),
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.global_status, "WORST_REGIME_SUPPORT_BOUND_ONLY");
        assert_eq!(result.worst_regime_threshold.unwrap().numerator, 1);
    }

    #[test]
    fn unknown_regime_threshold_is_the_worst_of_all_declared_regime_thresholds() {
        let dry = regime("dry", vec![
            sample("d1", "du1", 1),
            sample("d2", "du2", 4),
            sample("d3", "du3", 8),
        ]);
        let wet = regime("wet", vec![
            sample("w1", "wu1", 1),
            sample("w2", "wu2", 3),
            sample("w3", "wu3", 7),
        ]);
        let input = corpus(
            vec![wet, dry],
            Rational { numerator: 1, denominator: 4 },
            None,
        );
        let result = evaluate(&input).unwrap();
        assert_eq!(result.worst_regime_threshold.unwrap().numerator, 8);
        assert_eq!(result.worst_regime_ids, vec!["dry"]);
    }

    #[test]
    fn missing_planned_sample_is_rejected_even_when_regime_is_present() {
        let dry = regime("dry", vec![
            sample("d1", "du1", 1),
            sample("d2", "du2", 4),
            sample("d3", "du3", 8),
        ]);
        let mut input = corpus(vec![dry], Rational { numerator: 1, denominator: 4 }, None);
        input.regimes[0].samples.pop();
        input.regimes[0].samples_sha256 = digest_samples(&input.regimes[0].samples).unwrap();
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn post_hoc_regime_or_score_selection_is_rejected() {
        let dry = regime("dry", vec![
            sample("d1", "du1", 1),
            sample("d2", "du2", 4),
            sample("d3", "du3", 8),
        ]);
        let mut input = corpus(vec![dry], Rational { numerator: 1, denominator: 4 }, None);
        input.selection_status = "selected_after_outcomes".to_owned();
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn repeated_replication_unit_is_not_double_counted() {
        let r = regime("dry", vec![
            sample("s1", "same-unit", 1),
            sample("s2", "same-unit", 4),
            sample("s3", "u3", 8),
        ]);
        let input = corpus(vec![r], Rational { numerator: 1, denominator: 4 }, None);
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn alpha_must_be_strictly_between_zero_and_one() {
        let r = regime("dry", vec![
            sample("s1", "u1", 1),
            sample("s2", "u2", 4),
            sample("s3", "u3", 8),
        ]);
        let input = corpus(vec![r], Rational { numerator: 0, denominator: 1 }, None);
        assert!(evaluate(&input).is_err());
    }

    #[test]
    fn no_physical_qualification_or_authority_is_granted() {
        let r = regime("dry", vec![
            sample("s1", "u1", 1),
            sample("s2", "u2", 4),
            sample("s3", "u3", 8),
        ]);
        let result = evaluate(&corpus(
            vec![r],
            Rational { numerator: 1, denominator: 4 },
            None,
        )).unwrap();
        assert!(!result.physical_qualification_granted);
        assert!(!result.authority_granted);
    }
}
