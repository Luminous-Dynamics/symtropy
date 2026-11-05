// SPDX-License-Identifier: AGPL-3.0-or-later
//! Decision-grade evaluation of WPT predictive intervals.
//!
//! This crate evaluates empirical coverage and interval sharpness on frozen
//! held-out records. It does not establish future-distribution guarantees,
//! authenticate evidence digests, or certify physical safety.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: &str = "wpt-predictive-uncertainty-v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationRecord {
    pub schema_version: String,
    pub record_id: String,
    pub split: String,
    pub regime_id: String,
    pub trajectory_group_id: String,
    pub model_digest: String,
    pub calibration_set_digest: String,
    pub test_set_digest: String,
    pub quantity_kind: String,
    pub unit: String,
    pub nominal_coverage: f64,
    pub prediction: f64,
    pub lower: f64,
    pub upper: f64,
    pub observed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StratumReport {
    pub count: usize,
    pub covered: usize,
    pub empirical_coverage: f64,
    pub mean_interval_width: f64,
    pub mean_interval_score: f64,
    pub nominal_coverage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvaluationReport {
    pub schema_version: String,
    pub test_records: usize,
    pub covered_records: usize,
    pub empirical_coverage: f64,
    pub mean_interval_width: f64,
    pub mean_interval_score: f64,
    pub nominal_coverage: f64,
    pub coverage_error: f64,
    pub by_regime: BTreeMap<String, StratumReport>,
    pub claim_ceiling: String,
}

pub fn evaluate(records: &[EvaluationRecord]) -> Result<EvaluationReport, String> {
    if records.is_empty() {
        return Err("evaluation requires at least one record".into());
    }

    let reference = &records[0];
    let mut ids = std::collections::BTreeSet::new();
    let mut count = 0usize;
    let mut covered = 0usize;
    let mut width_sum = 0.0;
    let mut interval_score_sum = 0.0;
    let mut nominal = None::<f64>;
    let mut regimes = BTreeMap::<String, (usize, usize, f64, f64, f64)>::new();

    for record in records {
        if record.schema_version != SCHEMA_VERSION {
            return Err(format!("{}: unsupported schema version", record.record_id));
        }
        if record.model_digest != reference.model_digest
            || record.calibration_set_digest != reference.calibration_set_digest
            || record.test_set_digest != reference.test_set_digest
            || record.quantity_kind != reference.quantity_kind
            || record.unit != reference.unit
        {
            return Err(
                "a report must contain one model, calibration set, test set, quantity kind and unit"
                    .into(),
            );
        }
        if record.split != "test" {
            return Err(format!("{}: evaluator accepts test records only", record.record_id));
        }
        if record.record_id.trim().is_empty()
            || record.regime_id.trim().is_empty()
            || record.trajectory_group_id.trim().is_empty()
            || record.model_digest.trim().is_empty()
            || record.calibration_set_digest.trim().is_empty()
            || record.test_set_digest.trim().is_empty()
            || record.quantity_kind.trim().is_empty()
            || record.unit.trim().is_empty()
            || !ids.insert(&record.record_id)
        {
            return Err("record identity, provenance and semantic fields must be non-empty and unique".into());
        }
        for (name, value) in [
            ("nominal_coverage", record.nominal_coverage),
            ("prediction", record.prediction),
            ("lower", record.lower),
            ("upper", record.upper),
            ("observed", record.observed),
        ] {
            if !value.is_finite() {
                return Err(format!("{}: {name} must be finite", record.record_id));
            }
        }
        if !(0.0 < record.nominal_coverage && record.nominal_coverage < 1.0)
            || record.lower > record.upper
            || record.prediction < record.lower
            || record.prediction > record.upper
        {
            return Err(format!("{}: invalid prediction interval or nominal coverage", record.record_id));
        }
        match nominal {
            Some(value) if (value - record.nominal_coverage).abs() > f64::EPSILON => {
                return Err("a report must use one nominal coverage level".into());
            }
            None => nominal = Some(record.nominal_coverage),
            _ => {}
        }

        let hit = record.lower <= record.observed && record.observed <= record.upper;
        let width = record.upper - record.lower;
        if !width.is_finite() {
            return Err(format!("{}: interval width overflow", record.record_id));
        }
        let alpha = 1.0 - record.nominal_coverage;
        let miss_penalty = if record.observed < record.lower {
            (2.0 / alpha) * (record.lower - record.observed)
        } else if record.observed > record.upper {
            (2.0 / alpha) * (record.observed - record.upper)
        } else {
            0.0
        };
        let interval_score = width + miss_penalty;
        if !interval_score.is_finite() {
            return Err(format!("{}: interval score overflow", record.record_id));
        }
        count += 1;
        covered += if hit { 1 } else { 0 };
        width_sum += width;
        interval_score_sum += interval_score;
        if !width_sum.is_finite() || !interval_score_sum.is_finite() {
            return Err("aggregate interval metric overflow".into());
        }
        let entry = regimes
            .entry(record.regime_id.clone())
            .or_insert((0, 0, 0.0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += if hit { 1 } else { 0 };
        entry.2 += width;
        entry.3 += interval_score;
        entry.4 = record.nominal_coverage;
        if !entry.2.is_finite() || !entry.3.is_finite() {
            return Err(format!("{}: regime aggregate metric overflow", record.regime_id));
        }
    }

    let nominal = nominal.expect("non-empty records establish nominal coverage");
    let by_regime = regimes
        .into_iter()
        .map(|(regime, (n, hits, widths, score_sum, target))| {
            (
                regime,
                StratumReport {
                    count: n,
                    covered: hits,
                    empirical_coverage: hits as f64 / n as f64,
                    mean_interval_width: widths / n as f64,
                    mean_interval_score: score_sum / n as f64,
                    nominal_coverage: target,
                },
            )
        })
        .collect();

    let empirical_coverage = covered as f64 / count as f64;
    Ok(EvaluationReport {
        schema_version: SCHEMA_VERSION.into(),
        test_records: count,
        covered_records: covered,
        empirical_coverage,
        mean_interval_width: width_sum / count as f64,
        mean_interval_score: interval_score_sum / count as f64,
        nominal_coverage: nominal,
        coverage_error: empirical_coverage - nominal,
        by_regime,
        claim_ceiling: "Empirical held-out coverage and sharpness on the supplied records only. No guarantee under distribution shift, dependent sampling, or future operating conditions; conformal guarantees require separately stated exchangeability assumptions.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, regime: &str, observed: f64, lower: f64, upper: f64) -> EvaluationRecord {
        EvaluationRecord {
            schema_version: SCHEMA_VERSION.into(),
            record_id: id.into(),
            split: "test".into(),
            regime_id: regime.into(),
            trajectory_group_id: format!("trajectory-{id}"),
            model_digest: "model-sha256".into(),
            calibration_set_digest: "calibration-sha256".into(),
            test_set_digest: "test-sha256".into(),
            quantity_kind: "USEFUL_DC_ENERGY".into(),
            unit: "Wh".into(),
            nominal_coverage: 0.95,
            prediction: (lower + upper) / 2.0,
            lower,
            upper,
            observed,
        }
    }

    #[test]
    fn reports_coverage_and_sharpness_by_regime() {
        let report = evaluate(&[
            record("a", "aligned", 5.0, 4.0, 6.0),
            record("b", "aligned", 8.0, 4.0, 6.0),
            record("c", "misaligned", 5.0, 4.5, 5.5),
        ]).unwrap();
        assert_eq!(report.test_records, 3);
        assert_eq!(report.covered_records, 2);
        assert!((report.empirical_coverage - 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(report.by_regime["aligned"].covered, 1);
        assert_eq!(report.by_regime["misaligned"].mean_interval_width, 1.0);
    }

    #[test]
    fn rejects_calibration_records_in_test_evaluator() {
        let mut item = record("a", "aligned", 5.0, 4.0, 6.0);
        item.split = "calibration".into();
        assert!(evaluate(&[item]).unwrap_err().contains("test records only"));
    }

    #[test]
    fn rejects_duplicate_ids_and_invalid_intervals() {
        let item = record("a", "aligned", 5.0, 4.0, 6.0);
        assert!(evaluate(&[item.clone(), item]).unwrap_err().contains("unique"));
        let mut bad = record("b", "aligned", 5.0, 6.0, 4.0);
        bad.prediction = 5.0;
        assert!(evaluate(&[bad]).unwrap_err().contains("invalid prediction interval"));
    }

    #[test]
    fn rejects_non_finite_observations_and_vacuous_nominal_values() {
        let mut bad = record("a", "aligned", 5.0, 4.0, 6.0);
        bad.observed = f64::NAN;
        assert!(evaluate(&[bad]).unwrap_err().contains("finite"));
        let mut bad = record("b", "aligned", 5.0, 4.0, 6.0);
        bad.nominal_coverage = 1.0;
        assert!(evaluate(&[bad]).unwrap_err().contains("invalid prediction interval"));
    }

    #[test]
    fn interval_score_penalizes_misses_without_rewarding_vacuous_width() {
        let report = evaluate(&[
            record("inside", "aligned", 5.0, 4.0, 6.0),
            record("below", "aligned", 3.0, 4.0, 6.0),
        ])
        .unwrap();
        assert!((report.mean_interval_score - 22.0).abs() < 1e-12);
        assert!((report.by_regime["aligned"].mean_interval_score - 22.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_mixed_model_calibration_test_or_physical_quantity_metadata() {
        let baseline = record("a", "aligned", 5.0, 4.0, 6.0);
        let mut mixed_model = record("b", "aligned", 5.0, 4.0, 6.0);
        mixed_model.model_digest = "another-model".into();
        assert!(evaluate(&[baseline.clone(), mixed_model]).unwrap_err().contains("one model"));

        let mut mixed_calibration = record("c", "aligned", 5.0, 4.0, 6.0);
        mixed_calibration.calibration_set_digest = "another-calibration".into();
        assert!(evaluate(&[baseline.clone(), mixed_calibration]).unwrap_err().contains("one model"));

        let mut mixed_unit = record("d", "aligned", 5.0, 4.0, 6.0);
        mixed_unit.unit = "kWh".into();
        assert!(evaluate(&[baseline.clone(), mixed_unit]).unwrap_err().contains("one model"));

        let mut mixed_quantity = record("e", "aligned", 5.0, 4.0, 6.0);
        mixed_quantity.quantity_kind = "USEFUL_AC_ENERGY".into();
        assert!(evaluate(&[baseline.clone(), mixed_quantity]).unwrap_err().contains("one model"));

        let mut mixed_test = record("f", "aligned", 5.0, 4.0, 6.0);
        mixed_test.test_set_digest = "another-test".into();
        assert!(evaluate(&[baseline, mixed_test]).unwrap_err().contains("one model"));
    }

    #[test]
    fn rejects_mixed_nominal_levels() {
        let a = record("a", "aligned", 5.0, 4.0, 6.0);
        let mut b = record("b", "aligned", 5.0, 4.0, 6.0);
        b.nominal_coverage = 0.90;
        assert!(evaluate(&[a.clone(), b]).unwrap_err().contains("one nominal"));
    }
}
