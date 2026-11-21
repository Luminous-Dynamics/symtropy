//! Fail-closed composition of held-out correlation audit and sensor subset selection.
//!
//! The selector is not invoked unless every pair's simultaneous confidence
//! interval is contained in the declared correlation envelope. This is an
//! offline qualification gate: callers remain responsible for the freshness,
//! environment and independence assumptions of the residual holdout blocks.

use crate::correlated::CorrelatedRangeAction;
use crate::correlation_audit::{
    audit_correlation_envelope, CorrelationAuditDisposition, CorrelationAuditError,
    CorrelationModelAudit, CorrelationResidualBlock,
};
use crate::correlation_envelope::{
    robust_select_correlation_envelope, CorrelationEnvelopeError, CorrelationScenario,
    RobustCorrelationSelection,
};

#[derive(Clone, Debug, PartialEq)]
pub struct AuditedCorrelationPlan {
    pub audit: CorrelationModelAudit,
    pub selection: RobustCorrelationSelection,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AuditedCorrelationPlanError {
    Audit(CorrelationAuditError),
    ModelNotQualified(CorrelationAuditDisposition),
    Selection(CorrelationEnvelopeError),
}

impl std::fmt::Display for AuditedCorrelationPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Audit(error) => write!(f, "correlation audit failed: {error}"),
            Self::ModelNotQualified(disposition) => {
                write!(f, "correlation model not qualified for planning: {disposition:?}")
            }
            Self::Selection(error) => write!(f, "robust correlation selection failed: {error}"),
        }
    }
}
impl std::error::Error for AuditedCorrelationPlanError {}

/// Audit the candidate correlation envelope first; select only after a
/// familywise-confidence consistency result. Out-of-model and indeterminate
/// outcomes both fail closed and do not return a plan.
pub fn select_with_audited_correlation(
    actions: &[CorrelatedRangeAction],
    scenarios: &[CorrelationScenario],
    held_out_blocks: &[CorrelationResidualBlock],
    familywise_alpha: f64,
    budget_units: u32,
) -> Result<AuditedCorrelationPlan, AuditedCorrelationPlanError> {
    let audit = audit_correlation_envelope(
        actions,
        scenarios,
        held_out_blocks,
        familywise_alpha,
    )
    .map_err(AuditedCorrelationPlanError::Audit)?;

    if audit.disposition != CorrelationAuditDisposition::ConsistentAtDeclaredConfidence {
        return Err(AuditedCorrelationPlanError::ModelNotQualified(
            audit.disposition,
        ));
    }

    let selection = robust_select_correlation_envelope(actions, scenarios, budget_units)
        .map_err(AuditedCorrelationPlanError::Selection)?;

    Ok(AuditedCorrelationPlan { audit, selection })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn actions() -> Vec<CorrelatedRangeAction> {
        vec![
            CorrelatedRangeAction::independent("a-east", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b-east-copy", 0.04, 1.0, 1),
            CorrelatedRangeAction::independent("c-north", std::f64::consts::FRAC_PI_2, 1.0, 1),
            CorrelatedRangeAction::independent("d-diagonal", std::f64::consts::FRAC_PI_4, 1.0, 1),
        ]
    }

    fn complete_scenario(id: &str, rho: f64, ids: &[&str]) -> CorrelationScenario {
        let mut pairs = Vec::new();
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                pairs.push(crate::correlation_envelope::PairwiseCorrelation {
                    action_a: ids[i].to_owned(),
                    action_b: ids[j].to_owned(),
                    rho,
                });
            }
        }
        CorrelationScenario {
            scenario_id: id.to_owned(),
            pairs,
        }
    }

    fn scenarios() -> Vec<CorrelationScenario> {
        let ids = ["a-east", "b-east-copy", "c-north", "d-diagonal"];
        vec![
            complete_scenario("negative-overlap", -0.2, &ids),
            complete_scenario("positive-overlap", 0.2, &ids),
        ]
    }

    fn independent_blocks(count: usize) -> Vec<CorrelationResidualBlock> {
        (0..count)
            .map(|index| {
                let angle = 2.0 * std::f64::consts::PI * index as f64 / count as f64;
                CorrelationResidualBlock {
                    block_id: format!("holdout-{index:03}"),
                    residuals: BTreeMap::from([
                        ("a-east".to_owned(), angle.sin()),
                        ("b-east-copy".to_owned(), (2.0 * angle).sin()),
                        ("c-north".to_owned(), (3.0 * angle).sin()),
                        ("d-diagonal".to_owned(), (4.0 * angle).sin()),
                    ]),
                }
            })
            .collect()
    }

    fn misspecified_blocks(count: usize) -> Vec<CorrelationResidualBlock> {
        (0..count)
            .map(|index| {
                let angle = 2.0 * std::f64::consts::PI * index as f64 / count as f64;
                let a = angle.sin();
                CorrelationResidualBlock {
                    block_id: format!("holdout-{index:03}"),
                    residuals: BTreeMap::from([
                        ("a-east".to_owned(), a),
                        ("b-east-copy".to_owned(), 0.95 * a + 0.05 * (3.0 * angle).sin()),
                        ("c-north".to_owned(), (2.0 * angle).sin()),
                        ("d-diagonal".to_owned(), (4.0 * angle).sin()),
                    ]),
                }
            })
            .collect()
    }

    #[test]
    fn consistent_holdout_enables_worst_case_sensor_selection() {
        let result = select_with_audited_correlation(
            &actions(),
            &scenarios(),
            &independent_blocks(200),
            0.05,
            2,
        )
        .unwrap();

        assert_eq!(
            result.audit.disposition,
            CorrelationAuditDisposition::ConsistentAtDeclaredConfidence
        );
        assert_eq!(result.selection.total_cost_units, 2);
        assert_eq!(result.selection.envelope.worst_case_rank, 2);
    }

    #[test]
    fn out_of_model_holdout_blocks_sensor_selection() {
        let result = select_with_audited_correlation(
            &actions(),
            &scenarios(),
            &misspecified_blocks(200),
            0.05,
            2,
        );
        assert!(matches!(
            result,
            Err(AuditedCorrelationPlanError::ModelNotQualified(
                CorrelationAuditDisposition::OutOfModel
            ))
        ));
    }

    #[test]
    fn indeterminate_holdout_blocks_sensor_selection() {
        let result = select_with_audited_correlation(
            &actions(),
            &scenarios(),
            &independent_blocks(5),
            0.05,
            2,
        );
        assert!(matches!(
            result,
            Err(AuditedCorrelationPlanError::ModelNotQualified(
                CorrelationAuditDisposition::Indeterminate
            ))
        ));
    }
}
