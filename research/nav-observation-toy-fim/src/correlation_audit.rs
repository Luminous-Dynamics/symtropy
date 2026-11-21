//! Held-out audit of whether observed residual correlations fit a declared envelope.
//!
//! Residual blocks must be non-overlapping and independent enough for the
//! caller's evaluation design. The audit uses Fisher-z intervals with
//! Bonferroni correction across candidate pairs. It does not prove Gaussianity,
//! stationarity, independence of blocks, or continuous-time model coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::correlated::CorrelatedRangeAction;
use crate::correlation_envelope::{
    evaluate_correlation_envelope, CorrelationEnvelopeError, CorrelationScenario,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelationResidualBlock {
    pub block_id: String,
    pub residuals: BTreeMap<String, f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PairAuditDisposition {
    ConsistentWithEnvelope,
    ModelMisspecificationDetected,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairCorrelationAudit {
    pub action_a: String,
    pub action_b: String,
    pub sample_count: usize,
    pub empirical_correlation: Option<f64>,
    pub confidence_interval: Option<[f64; 2]>,
    pub declared_interval: [f64; 2],
    pub disposition: PairAuditDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CorrelationAuditDisposition {
    ConsistentAtDeclaredConfidence,
    OutOfModel,
    Indeterminate,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorrelationModelAudit {
    pub familywise_alpha: f64,
    pub simultaneous_confidence: f64,
    pub pair_audits: Vec<PairCorrelationAudit>,
    pub disposition: CorrelationAuditDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CorrelationAuditError {
    InvalidAlpha,
    EmptyResidualBlocks,
    EmptyBlockId,
    DuplicateBlockId(String),
    UnknownResidualSource(String),
    NonFiniteResidual(String, String),
    InvalidDeclaredModel(CorrelationEnvelopeError),
    TooFewActions,
}

impl fmt::Display for CorrelationAuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAlpha => write!(f, "familywise alpha must be finite and strictly between 0 and 1"),
            Self::EmptyResidualBlocks => write!(f, "at least one residual block is required"),
            Self::EmptyBlockId => write!(f, "residual block ID must not be empty"),
            Self::DuplicateBlockId(id) => write!(f, "duplicate residual block ID: {id}"),
            Self::UnknownResidualSource(id) => write!(f, "residual block contains an undeclared source: {id}"),
            Self::NonFiniteResidual(block, source) => write!(f, "non-finite residual for source {source} in block {block}"),
            Self::InvalidDeclaredModel(error) => write!(f, "declared correlation model is invalid: {error}"),
            Self::TooFewActions => write!(f, "at least two actions are required for a correlation audit"),
        }
    }
}
impl std::error::Error for CorrelationAuditError {}

fn canonical_pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    }
}

// Acklam inverse-normal approximation.
fn inverse_standard_normal(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1, 2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2, 1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1, 2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1, 1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2, 6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3, -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838, -2.549_732_539_343_734,
        4.374_664_141_464_968, 2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3, 3.224_671_290_700_398e-1,
        2.445_134_137_142_996, 3.754_408_661_907_416,
    ];
    const LOW: f64 = 0.024_25;
    const HIGH: f64 = 1.0 - LOW;

    if p <= 0.0 { return f64::NEG_INFINITY; }
    if p >= 1.0 { return f64::INFINITY; }
    if p < LOW {
        let q = (-2.0 * p.ln()).sqrt();
        return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }
    if p > HIGH {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        return -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0);
    }
    let q = p - 0.5;
    let r = q * q;
    (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
        / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
}

fn correlation_confidence_interval(xs: &[f64], ys: &[f64], zcrit: f64) -> Option<(f64, [f64; 2])> {
    if xs.len() != ys.len() || xs.len() < 4 { return None; }
    let n = xs.len();
    let mx = xs.iter().sum::<f64>() / n as f64;
    let my = ys.iter().sum::<f64>() / n as f64;
    let mut xx = 0.0;
    let mut yy = 0.0;
    let mut xy = 0.0;
    for (x, y) in xs.iter().zip(ys) {
        let dx = x - mx;
        let dy = y - my;
        xx += dx * dx;
        yy += dy * dy;
        xy += dx * dy;
    }
    if !xx.is_finite() || !yy.is_finite() || !xy.is_finite() || xx <= 0.0 || yy <= 0.0 {
        return None;
    }
    let raw_r = xy / (xx * yy).sqrt();
    if !raw_r.is_finite() { return None; }
    let r = raw_r.clamp(-1.0 + 1.0e-12, 1.0 - 1.0e-12);
    let se = 1.0 / ((n - 3) as f64).sqrt();
    let centre = r.atanh();
    Some((r, [(centre - zcrit * se).tanh(), (centre + zcrit * se).tanh()]))
}

/// Audit held-out residual correlations against a finite declared scenario set.
///
/// Blocks are assumed independent by design. Missing samples are skipped, not
/// imputed. A confidence interval must be wholly inside the declared interval
/// to pass; a disjoint interval detects misspecification; overlap without
/// containment remains INDETERMINATE. Bonferroni controls familywise alpha.
pub fn audit_correlation_envelope(
    actions: &[CorrelatedRangeAction],
    scenarios: &[CorrelationScenario],
    blocks: &[CorrelationResidualBlock],
    familywise_alpha: f64,
) -> Result<CorrelationModelAudit, CorrelationAuditError> {
    if !familywise_alpha.is_finite() || familywise_alpha <= 0.0 || familywise_alpha >= 1.0 {
        return Err(CorrelationAuditError::InvalidAlpha);
    }
    if blocks.is_empty() { return Err(CorrelationAuditError::EmptyResidualBlocks); }
    if actions.len() < 2 { return Err(CorrelationAuditError::TooFewActions); }

    evaluate_correlation_envelope(actions, scenarios)
        .map_err(CorrelationAuditError::InvalidDeclaredModel)?;

    let source_ids: BTreeSet<&str> = actions.iter().map(|a| a.id.as_str()).collect();
    let mut block_ids = BTreeSet::new();
    for block in blocks {
        if block.block_id.trim().is_empty() { return Err(CorrelationAuditError::EmptyBlockId); }
        if !block_ids.insert(block.block_id.as_str()) {
            return Err(CorrelationAuditError::DuplicateBlockId(block.block_id.clone()));
        }
        for (source, value) in &block.residuals {
            if !source_ids.contains(source.as_str()) {
                return Err(CorrelationAuditError::UnknownResidualSource(source.clone()));
            }
            if !value.is_finite() {
                return Err(CorrelationAuditError::NonFiniteResidual(block.block_id.clone(), source.clone()));
            }
        }
    }

    let pair_count = actions.len() * (actions.len() - 1) / 2;
    let zcrit = inverse_standard_normal(1.0 - familywise_alpha / (2.0 * pair_count as f64));
    let mut pair_audits = Vec::with_capacity(pair_count);

    for i in 0..actions.len() {
        for j in (i + 1)..actions.len() {
            let (a, b) = canonical_pair(&actions[i].id, &actions[j].id);
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for block in blocks {
                if let (Some(x), Some(y)) = (block.residuals.get(&a), block.residuals.get(&b)) {
                    xs.push(*x);
                    ys.push(*y);
                }
            }

            let mut declared_low = f64::INFINITY;
            let mut declared_high = f64::NEG_INFINITY;
            for scenario in scenarios {
                let pair = scenario.pairs.iter()
                    .find(|p| canonical_pair(&p.action_a, &p.action_b) == (a.clone(), b.clone()))
                    .expect("full pair graph validated by envelope evaluator");
                declared_low = declared_low.min(pair.rho);
                declared_high = declared_high.max(pair.rho);
            }

            let (empirical_correlation, confidence_interval, disposition) =
                match correlation_confidence_interval(&xs, &ys, zcrit) {
                    None => (None, None, PairAuditDisposition::Indeterminate),
                    Some((rho, interval)) => {
                        let d = if interval[1] < declared_low || interval[0] > declared_high {
                            PairAuditDisposition::ModelMisspecificationDetected
                        } else if interval[0] >= declared_low && interval[1] <= declared_high {
                            PairAuditDisposition::ConsistentWithEnvelope
                        } else {
                            PairAuditDisposition::Indeterminate
                        };
                        (Some(rho), Some(interval), d)
                    }
                };

            pair_audits.push(PairCorrelationAudit {
                action_a: a,
                action_b: b,
                sample_count: xs.len(),
                empirical_correlation,
                confidence_interval,
                declared_interval: [declared_low, declared_high],
                disposition,
            });
        }
    }

    let disposition = if pair_audits.iter().any(|p| p.disposition == PairAuditDisposition::ModelMisspecificationDetected) {
        CorrelationAuditDisposition::OutOfModel
    } else if pair_audits.iter().all(|p| p.disposition == PairAuditDisposition::ConsistentWithEnvelope) {
        CorrelationAuditDisposition::ConsistentAtDeclaredConfidence
    } else {
        CorrelationAuditDisposition::Indeterminate
    };

    Ok(CorrelationModelAudit {
        familywise_alpha,
        simultaneous_confidence: 1.0 - familywise_alpha,
        pair_audits,
        disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::correlation_envelope::PairwiseCorrelation;

    fn actions() -> Vec<CorrelatedRangeAction> {
        vec![
            CorrelatedRangeAction::independent("a", 0.0, 1.0, 1),
            CorrelatedRangeAction::independent("b", std::f64::consts::FRAC_PI_2, 1.0, 1),
        ]
    }

    fn scenarios(low: f64, high: f64) -> Vec<CorrelationScenario> {
        vec![
            CorrelationScenario { scenario_id: "low".into(), pairs: vec![PairwiseCorrelation { action_a: "a".into(), action_b: "b".into(), rho: low }] },
            CorrelationScenario { scenario_id: "high".into(), pairs: vec![PairwiseCorrelation { action_a: "a".into(), action_b: "b".into(), rho: high }] },
        ]
    }

    fn blocks(count: usize, correlated: bool) -> Vec<CorrelationResidualBlock> {
        (0..count).map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / count as f64;
            let x = angle.sin();
            let y = if correlated { 0.95 * x + 0.05 * (3.0 * angle).sin() } else { angle.cos() };
            CorrelationResidualBlock {
                block_id: format!("block-{i:03}"),
                residuals: BTreeMap::from([("a".into(), x), ("b".into(), y)]),
            }
        }).collect()
    }

    #[test]
    fn held_out_correlation_outside_declared_envelope_is_detected() {
        let result = audit_correlation_envelope(&actions(), &scenarios(-0.2, 0.2), &blocks(100, true), 0.05).unwrap();
        assert_eq!(result.disposition, CorrelationAuditDisposition::OutOfModel);
        assert_eq!(result.pair_audits[0].disposition, PairAuditDisposition::ModelMisspecificationDetected);
        assert!(result.pair_audits[0].empirical_correlation.unwrap() > 0.9);
    }

    #[test]
    fn held_out_correlation_inside_declared_envelope_is_consistent() {
        let result = audit_correlation_envelope(&actions(), &scenarios(-0.25, 0.25), &blocks(100, false), 0.05).unwrap();
        assert_eq!(result.disposition, CorrelationAuditDisposition::ConsistentAtDeclaredConfidence);
        let interval = result.pair_audits[0].confidence_interval.unwrap();
        assert!(interval[0] > -0.25 && interval[1] < 0.25);
    }

    #[test]
    fn too_few_blocks_are_indeterminate_not_assumed_independent() {
        let result = audit_correlation_envelope(&actions(), &scenarios(-1.0, 1.0), &blocks(3, false), 0.05).unwrap();
        assert_eq!(result.disposition, CorrelationAuditDisposition::Indeterminate);
        assert_eq!(result.pair_audits[0].disposition, PairAuditDisposition::Indeterminate);
    }

    #[test]
    fn constant_residual_source_is_indeterminate() {
        let samples = (0..30).map(|i| CorrelationResidualBlock {
            block_id: format!("block-{i:03}"),
            residuals: BTreeMap::from([("a".into(), i as f64), ("b".into(), 1.0)]),
        }).collect::<Vec<_>>();
        let result = audit_correlation_envelope(&actions(), &scenarios(-1.0, 1.0), &samples, 0.05).unwrap();
        assert_eq!(result.disposition, CorrelationAuditDisposition::Indeterminate);
    }

    #[test]
    fn missing_samples_are_not_imputed() {
        let mut samples = blocks(100, false);
        for block in samples.iter_mut().take(95) { block.residuals.remove("b"); }
        let result = audit_correlation_envelope(&actions(), &scenarios(-1.0, 1.0), &samples, 0.05).unwrap();
        assert_eq!(result.pair_audits[0].sample_count, 5);
        assert_eq!(result.disposition, CorrelationAuditDisposition::Indeterminate);
    }

    #[test]
    fn duplicate_blocks_unknown_sources_and_non_finite_residuals_fail_closed() {
        let mut duplicate = blocks(4, false);
        duplicate[1].block_id = duplicate[0].block_id.clone();
        assert!(matches!(audit_correlation_envelope(&actions(), &scenarios(-1.0, 1.0), &duplicate, 0.05), Err(CorrelationAuditError::DuplicateBlockId(_))));

        let mut unknown = blocks(4, false);
        unknown[0].residuals.insert("ghost".into(), 0.0);
        assert!(matches!(audit_correlation_envelope(&actions(), &scenarios(-1.0, 1.0), &unknown, 0.05), Err(CorrelationAuditError::UnknownResidualSource(_))));

        let mut invalid = blocks(4, false);
        invalid[0].residuals.insert("a".into(), f64::NAN);
        assert!(matches!(audit_correlation_envelope(&actions(), &scenarios(-1.0, 1.0), &invalid, 0.05), Err(CorrelationAuditError::NonFiniteResidual(_, _))));
    }

    #[test]
    fn invalid_alpha_and_invalid_declared_pair_graph_are_rejected() {
        assert_eq!(audit_correlation_envelope(&actions(), &scenarios(-1.0, 1.0), &blocks(10, false), 0.0), Err(CorrelationAuditError::InvalidAlpha));
        let incomplete = CorrelationScenario { scenario_id: "incomplete".into(), pairs: vec![] };
        assert!(matches!(audit_correlation_envelope(&actions(), &[incomplete], &blocks(10, false), 0.05), Err(CorrelationAuditError::InvalidDeclaredModel(_))));
    }
}
