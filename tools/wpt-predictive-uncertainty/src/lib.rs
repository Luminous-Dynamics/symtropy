// SPDX-License-Identifier: AGPL-3.0-or-later
//! Decision-grade evaluation of WPT predictive intervals.
//!
//! This crate evaluates empirical coverage and interval sharpness on frozen
//! held-out records. It does not establish future-distribution guarantees,
//! authenticate evidence digests, or certify physical safety.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: &str = "wpt-predictive-uncertainty-v1";

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

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
    pub trajectory_groups: usize,
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
    pub unique_trajectory_groups: usize,
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
    let mut trajectory_groups = std::collections::BTreeSet::new();
    let mut count = 0usize;
    let mut covered = 0usize;
    let mut width_sum = 0.0;
    let mut interval_score_sum = 0.0;
    let mut nominal = None::<f64>;
    let mut regimes = BTreeMap::<
        String,
        (
            usize,
            usize,
            f64,
            f64,
            f64,
            std::collections::BTreeSet<String>,
        ),
    >::new();

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
            return Err(format!(
                "{}: evaluator accepts test records only",
                record.record_id
            ));
        }
        if record.record_id.trim().is_empty()
            || record.regime_id.trim().is_empty()
            || record.trajectory_group_id.trim().is_empty()
            || record.model_digest.trim().is_empty()
            || record.calibration_set_digest.trim().is_empty()
            || record.test_set_digest.trim().is_empty()
            || record.quantity_kind.trim().is_empty()
            || record.unit.trim().is_empty()
            || !is_sha256_digest(&record.model_digest)
            || !is_sha256_digest(&record.calibration_set_digest)
            || !is_sha256_digest(&record.test_set_digest)
            || !ids.insert(&record.record_id)
        {
            return Err(
                "record identity, provenance and semantic fields must be non-empty and unique"
                    .into(),
            );
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
            return Err(format!(
                "{}: invalid prediction interval or nominal coverage",
                record.record_id
            ));
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
        trajectory_groups.insert(record.trajectory_group_id.clone());
        covered += if hit { 1 } else { 0 };
        width_sum += width;
        interval_score_sum += interval_score;
        if !width_sum.is_finite() || !interval_score_sum.is_finite() {
            return Err("aggregate interval metric overflow".into());
        }
        let entry = regimes
            .entry(record.regime_id.clone())
            .or_insert_with(|| (0, 0, 0.0, 0.0, 0.0, std::collections::BTreeSet::new()));
        entry.0 += 1;
        entry.1 += if hit { 1 } else { 0 };
        entry.2 += width;
        entry.3 += interval_score;
        entry.4 = record.nominal_coverage;
        entry.5.insert(record.trajectory_group_id.clone());
        if !entry.2.is_finite() || !entry.3.is_finite() {
            return Err(format!(
                "{}: regime aggregate metric overflow",
                record.regime_id
            ));
        }
    }

    let nominal = nominal.expect("non-empty records establish nominal coverage");
    let by_regime = regimes
        .into_iter()
        .map(|(regime, (n, hits, widths, score_sum, target, groups))| {
            (
                regime,
                StratumReport {
                    count: n,
                    trajectory_groups: groups.len(),
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
        unique_trajectory_groups: trajectory_groups.len(),
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

/// Direction of a hard operating-envelope constraint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConstraintSense {
    /// The entire qualified interval must be at or above the threshold.
    Minimum,
    /// The entire qualified interval must be at or below the threshold.
    Maximum,
}

/// Result for one interval-valued operating constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionConstraint {
    pub constraint_id: String,
    pub unit: String,
    pub sense: ConstraintSense,
    pub threshold: f64,
    pub lower: f64,
    pub upper: f64,
}

/// Disposition for one constraint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConstraintDisposition {
    Satisfied,
    Violated,
    Unresolved,
}

/// Fail-closed decision from all supplied hard constraints.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateDecision {
    /// Every supplied constraint is satisfied by the complete interval.
    Eligible,
    /// At least one complete interval is beyond a hard limit.
    Inhibit,
    /// No hard limit is definitely violated, but at least one is unresolved.
    NotEvaluable,
}

/// Auditable result for one constraint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstraintCheck {
    pub constraint_id: String,
    pub disposition: ConstraintDisposition,
    pub explanation: String,
}

/// Aggregate result. ELIGIBLE is not energization authorization: all independent
/// interlocks, identity, positioning and protection checks still apply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DecisionGateReport {
    pub decision: GateDecision,
    pub checks: Vec<ConstraintCheck>,
    pub action: String,
    pub claim_ceiling: String,
}

/// Evaluate predictive intervals against hard minimum/maximum constraints.
///
/// A constraint is satisfied only when its *entire* interval is on the allowed
/// side of the threshold. Overlap with the threshold is UNRESOLVED, not SAFE.
/// Empty, malformed, non-finite or duplicate inputs fail closed with an error.
pub fn evaluate_decision_gate(
    constraints: &[DecisionConstraint],
) -> Result<DecisionGateReport, String> {
    if constraints.is_empty() {
        return Err("decision gate requires at least one hard constraint".into());
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut checks = Vec::with_capacity(constraints.len());
    for constraint in constraints {
        if constraint.constraint_id.trim().is_empty()
            || constraint.unit.trim().is_empty()
            || !seen.insert(&constraint.constraint_id)
        {
            return Err(
                "constraint identity and unit must be non-empty; IDs must be unique".into(),
            );
        }
        for (name, value) in [
            ("threshold", constraint.threshold),
            ("lower", constraint.lower),
            ("upper", constraint.upper),
        ] {
            if !value.is_finite() {
                return Err(format!(
                    "{}: {name} must be finite",
                    constraint.constraint_id
                ));
            }
        }
        if constraint.lower > constraint.upper {
            return Err(format!(
                "{}: interval lower bound exceeds upper bound",
                constraint.constraint_id
            ));
        }

        let (disposition, explanation) = match constraint.sense {
            ConstraintSense::Minimum if constraint.lower >= constraint.threshold => (
                ConstraintDisposition::Satisfied,
                "entire interval meets or exceeds the required minimum".to_string(),
            ),
            ConstraintSense::Minimum if constraint.upper < constraint.threshold => (
                ConstraintDisposition::Violated,
                "entire interval is below the required minimum".to_string(),
            ),
            ConstraintSense::Minimum => (
                ConstraintDisposition::Unresolved,
                "interval overlaps the required minimum; feasibility is unresolved".to_string(),
            ),
            ConstraintSense::Maximum if constraint.upper <= constraint.threshold => (
                ConstraintDisposition::Satisfied,
                "entire interval is at or below the permitted maximum".to_string(),
            ),
            ConstraintSense::Maximum if constraint.lower > constraint.threshold => (
                ConstraintDisposition::Violated,
                "entire interval exceeds the permitted maximum".to_string(),
            ),
            ConstraintSense::Maximum => (
                ConstraintDisposition::Unresolved,
                "interval overlaps the permitted maximum; feasibility is unresolved".to_string(),
            ),
        };
        checks.push(ConstraintCheck {
            constraint_id: constraint.constraint_id.clone(),
            disposition,
            explanation,
        });
    }

    let decision = if checks
        .iter()
        .any(|check| check.disposition == ConstraintDisposition::Violated)
    {
        GateDecision::Inhibit
    } else if checks
        .iter()
        .any(|check| check.disposition == ConstraintDisposition::Unresolved)
    {
        GateDecision::NotEvaluable
    } else {
        GateDecision::Eligible
    };
    let action = match decision {
        GateDecision::Eligible => "ELIGIBLE_FOR_INDEPENDENT_INTERLOCK_AND_AUTHORIZATION_CHECKS",
        GateDecision::Inhibit => "INHIBIT_OR_ENTER_DECLARED_SAFE_STATE",
        GateDecision::NotEvaluable => "DO_NOT_INCREASE_POWER; DERATE_OR_RAMP_DOWN",
    }
    .to_string();

    Ok(DecisionGateReport {
        decision,
        checks,
        action,
        claim_ceiling: "Interval constraint screening only. ELIGIBLE is not permission to energize and does not replace independent protection, identity, positioning, exposure, thermal, foreign-object, communications-freshness or hardware interlocks.".into(),
    })
}

/// Result that combines interval feasibility with operating-regime qualification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegimeDecisionGateReport {
    pub regime_id: String,
    pub regime_supported: bool,
    pub decision: GateDecision,
    pub interval_gate: DecisionGateReport,
    pub action: String,
}

/// Evaluate hard constraints only inside a pre-qualified operating regime.
///
/// An unsupported regime is NOT_EVALUABLE even when the supplied intervals
/// appear to pass. A definite interval violation still returns INHIBIT.
pub fn evaluate_decision_gate_for_regime(
    regime_id: &str,
    qualified_regimes: &[String],
    constraints: &[DecisionConstraint],
) -> Result<RegimeDecisionGateReport, String> {
    if regime_id.trim().is_empty() || qualified_regimes.is_empty() {
        return Err("current regime and qualified regime set must be non-empty".into());
    }
    let mut seen = std::collections::BTreeSet::<String>::new();
    for qualified in qualified_regimes {
        if qualified.trim().is_empty() || !seen.insert(qualified.clone()) {
            return Err("qualified regime IDs must be non-empty and unique".into());
        }
    }
    let regime_supported = seen.contains(regime_id);
    let mut interval_gate = evaluate_decision_gate(constraints)?;
    let decision = if interval_gate.decision == GateDecision::Inhibit {
        GateDecision::Inhibit
    } else if !regime_supported || interval_gate.decision == GateDecision::NotEvaluable {
        GateDecision::NotEvaluable
    } else {
        GateDecision::Eligible
    };
    interval_gate.decision = decision;
    interval_gate.action = match decision {
        GateDecision::Eligible => "ELIGIBLE_FOR_INDEPENDENT_INTERLOCK_AND_AUTHORIZATION_CHECKS",
        GateDecision::Inhibit => "INHIBIT_OR_ENTER_DECLARED_SAFE_STATE",
        GateDecision::NotEvaluable => "DO_NOT_INCREASE_POWER; DERATE_OR_RAMP_DOWN",
    }
    .to_string();
    let action = interval_gate.action.clone();
    Ok(RegimeDecisionGateReport {
        regime_id: regime_id.to_string(),
        regime_supported,
        decision,
        interval_gate,
        action,
    })
}

#[cfg(test)]
mod decision_gate_tests {
    use super::*;

    fn constraint(
        id: &str,
        sense: ConstraintSense,
        threshold: f64,
        lower: f64,
        upper: f64,
    ) -> DecisionConstraint {
        DecisionConstraint {
            constraint_id: id.into(),
            unit: "W".into(),
            sense,
            threshold,
            lower,
            upper,
        }
    }

    #[test]
    fn complete_intervals_on_allowed_side_are_eligible() {
        let result = evaluate_decision_gate(&[
            constraint(
                "minimum_service",
                ConstraintSense::Minimum,
                100.0,
                102.0,
                108.0,
            ),
            constraint(
                "maximum_temperature",
                ConstraintSense::Maximum,
                80.0,
                70.0,
                75.0,
            ),
        ])
        .unwrap();
        assert_eq!(result.decision, GateDecision::Eligible);
    }

    #[test]
    fn threshold_overlap_is_not_silently_accepted() {
        let result = evaluate_decision_gate(&[constraint(
            "minimum_service",
            ConstraintSense::Minimum,
            100.0,
            98.0,
            103.0,
        )])
        .unwrap();
        assert_eq!(result.decision, GateDecision::NotEvaluable);
        assert_eq!(
            result.checks[0].disposition,
            ConstraintDisposition::Unresolved
        );
    }

    #[test]
    fn definite_limit_breach_inhibits() {
        let result = evaluate_decision_gate(&[constraint(
            "maximum_temperature",
            ConstraintSense::Maximum,
            80.0,
            82.0,
            85.0,
        )])
        .unwrap();
        assert_eq!(result.decision, GateDecision::Inhibit);
    }

    #[test]
    fn definite_breach_dominates_other_unresolved_constraints() {
        let result = evaluate_decision_gate(&[
            constraint(
                "minimum_service",
                ConstraintSense::Minimum,
                100.0,
                98.0,
                103.0,
            ),
            constraint(
                "maximum_temperature",
                ConstraintSense::Maximum,
                80.0,
                82.0,
                85.0,
            ),
        ])
        .unwrap();
        assert_eq!(result.decision, GateDecision::Inhibit);
    }

    #[test]
    fn unsupported_regime_blocks_otherwise_eligible_intervals() {
        let result = evaluate_decision_gate_for_regime(
            "hot_misaligned",
            &["aligned_cold".to_string()],
            &[constraint(
                "maximum_temperature",
                ConstraintSense::Maximum,
                80.0,
                70.0,
                75.0,
            )],
        )
        .unwrap();
        assert!(!result.regime_supported);
        assert_eq!(result.decision, GateDecision::NotEvaluable);
    }

    #[test]
    fn definite_violation_still_inhibits_in_unsupported_regime() {
        let result = evaluate_decision_gate_for_regime(
            "unknown",
            &["aligned_cold".to_string()],
            &[constraint(
                "maximum_temperature",
                ConstraintSense::Maximum,
                80.0,
                82.0,
                85.0,
            )],
        )
        .unwrap();
        assert_eq!(result.decision, GateDecision::Inhibit);
    }

    #[test]
    fn malformed_gate_inputs_fail_closed() {
        assert!(evaluate_decision_gate(&[]).is_err());
        assert!(
            evaluate_decision_gate(&[constraint(
                "bad",
                ConstraintSense::Minimum,
                1.0,
                f64::NAN,
                2.0
            ),])
            .is_err()
        );
        assert!(
            evaluate_decision_gate(&[constraint("bad", ConstraintSense::Minimum, 1.0, 2.0, 1.0),])
                .is_err()
        );
        assert!(
            evaluate_decision_gate(&[
                constraint("duplicate", ConstraintSense::Minimum, 1.0, 1.0, 2.0),
                constraint("duplicate", ConstraintSense::Maximum, 3.0, 1.0, 2.0),
            ])
            .is_err()
        );
    }
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
            model_digest: "a".repeat(64),
            calibration_set_digest: "b".repeat(64),
            test_set_digest: "c".repeat(64),
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
        ])
        .unwrap();
        assert_eq!(report.test_records, 3);
        assert_eq!(report.unique_trajectory_groups, 3);
        assert_eq!(report.covered_records, 2);
        assert_eq!(report.by_regime["aligned"].trajectory_groups, 2);
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
        assert!(
            evaluate(&[item.clone(), item])
                .unwrap_err()
                .contains("unique")
        );
        let mut bad = record("b", "aligned", 5.0, 6.0, 4.0);
        bad.prediction = 5.0;
        assert!(
            evaluate(&[bad])
                .unwrap_err()
                .contains("invalid prediction interval")
        );
    }

    #[test]
    fn rejects_non_finite_observations_and_vacuous_nominal_values() {
        let mut bad = record("a", "aligned", 5.0, 4.0, 6.0);
        bad.observed = f64::NAN;
        assert!(evaluate(&[bad]).unwrap_err().contains("finite"));
        let mut bad = record("b", "aligned", 5.0, 4.0, 6.0);
        bad.nominal_coverage = 1.0;
        assert!(
            evaluate(&[bad])
                .unwrap_err()
                .contains("invalid prediction interval")
        );
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
        assert!(
            evaluate(&[baseline.clone(), mixed_model])
                .unwrap_err()
                .contains("one model")
        );

        let mut mixed_calibration = record("c", "aligned", 5.0, 4.0, 6.0);
        mixed_calibration.calibration_set_digest = "another-calibration".into();
        assert!(
            evaluate(&[baseline.clone(), mixed_calibration])
                .unwrap_err()
                .contains("one model")
        );

        let mut mixed_unit = record("d", "aligned", 5.0, 4.0, 6.0);
        mixed_unit.unit = "kWh".into();
        assert!(
            evaluate(&[baseline.clone(), mixed_unit])
                .unwrap_err()
                .contains("one model")
        );

        let mut mixed_quantity = record("e", "aligned", 5.0, 4.0, 6.0);
        mixed_quantity.quantity_kind = "USEFUL_AC_ENERGY".into();
        assert!(
            evaluate(&[baseline.clone(), mixed_quantity])
                .unwrap_err()
                .contains("one model")
        );

        let mut mixed_test = record("f", "aligned", 5.0, 4.0, 6.0);
        mixed_test.test_set_digest = "another-test".into();
        assert!(
            evaluate(&[baseline, mixed_test])
                .unwrap_err()
                .contains("one model")
        );
    }

    #[test]
    fn rejects_malformed_provenance_digests() {
        let mut short = record("a", "aligned", 5.0, 4.0, 6.0);
        short.model_digest = "model-demo".into();
        assert!(evaluate(&[short]).unwrap_err().contains("provenance"));

        let mut uppercase = record("b", "aligned", 5.0, 4.0, 6.0);
        uppercase.calibration_set_digest = "A".repeat(64);
        assert!(evaluate(&[uppercase]).unwrap_err().contains("provenance"));

        let mut malformed_test = record("c", "aligned", 5.0, 4.0, 6.0);
        malformed_test.test_set_digest = "g".repeat(64);
        assert!(
            evaluate(&[malformed_test])
                .unwrap_err()
                .contains("provenance")
        );
    }

    #[test]
    fn rejects_mixed_nominal_levels() {
        let a = record("a", "aligned", 5.0, 4.0, 6.0);
        let mut b = record("b", "aligned", 5.0, 4.0, 6.0);
        b.nominal_coverage = 0.90;
        assert!(
            evaluate(&[a.clone(), b])
                .unwrap_err()
                .contains("one nominal")
        );
    }
}
