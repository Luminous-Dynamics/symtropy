//! Exact finite minimax-regret decision oracle.
//!
//! This is a deliberately small decision-qualification reference. It does not
//! infer probabilities over epistemic model hypotheses and does not replace
//! evidence qualification or causal-support checks upstream.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::StableId;

const DOMAIN: &[u8] = b"symtropy.road-decision-closure.v0.1\0";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ExactRational {
    pub numerator: i128,
    pub denominator: u128,
}

impl ExactRational {
    pub fn new(numerator: i128, denominator: u128) -> Result<Self, DecisionError> {
        if denominator == 0 {
            return Err(DecisionError::ZeroDenominator);
        }
        if numerator == 0 {
            return Ok(Self {
                numerator: 0,
                denominator: 1,
            });
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        let signed_divisor =
            i128::try_from(divisor).map_err(|_| DecisionError::ArithmeticOverflow)?;
        Ok(Self {
            numerator: numerator / signed_divisor,
            denominator: denominator / divisor,
        })
    }

    pub fn checked_cmp(&self, other: &Self) -> Result<Ordering, DecisionError> {
        let left_denominator = i128::try_from(other.denominator)
            .map_err(|_| DecisionError::ArithmeticOverflow)?;
        let right_denominator = i128::try_from(self.denominator)
            .map_err(|_| DecisionError::ArithmeticOverflow)?;
        let left = self
            .numerator
            .checked_mul(left_denominator)
            .ok_or(DecisionError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_denominator)
            .ok_or(DecisionError::ArithmeticOverflow)?;
        Ok(left.cmp(&right))
    }

    pub fn checked_sub(&self, other: &Self) -> Result<Self, DecisionError> {
        let left_denominator = i128::try_from(other.denominator)
            .map_err(|_| DecisionError::ArithmeticOverflow)?;
        let right_denominator = i128::try_from(self.denominator)
            .map_err(|_| DecisionError::ArithmeticOverflow)?;
        let left = self
            .numerator
            .checked_mul(left_denominator)
            .ok_or(DecisionError::ArithmeticOverflow)?;
        let right = other
            .numerator
            .checked_mul(right_denominator)
            .ok_or(DecisionError::ArithmeticOverflow)?;
        let numerator = left
            .checked_sub(right)
            .ok_or(DecisionError::ArithmeticOverflow)?;
        let denominator = self
            .denominator
            .checked_mul(other.denominator)
            .ok_or(DecisionError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutcomeSupport {
    Supported {
        utility: ExactRational,
        evidence_digest: [u8; 32],
    },
    Unsupported {
        reason_id: StableId,
    },
    NotIdentified {
        evidence_ref: StableId,
    },
    Incomplete {
        reason_id: StableId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisActionOutcome {
    pub action_id: StableId,
    pub support: OutcomeSupport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionHypothesis {
    pub hypothesis_id: StableId,
    /// Must identify the exact evidence/reference manifest qualifying this model.
    pub reference_manifest_digest: [u8; 32],
    pub outcomes: Vec<HypothesisActionOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionManifest {
    pub decision_id: StableId,
    pub version: StableId,
    pub actions: Vec<StableId>,
    pub hypotheses: Vec<DecisionHypothesis>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionStatus {
    DecisionStable,
    DecisionSensitive,
    NoRobustDecision,
    Unsupported,
    Incomplete,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportGapKind {
    Unsupported,
    NotIdentified,
    Incomplete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportGap {
    pub hypothesis_id: StableId,
    pub action_id: StableId,
    pub kind: SupportGapKind,
    pub reason_id: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisRegret {
    pub hypothesis_id: StableId,
    pub best_utility: ExactRational,
    pub best_actions: Vec<StableId>,
    pub regrets: Vec<(StableId, ExactRational)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionRegret {
    pub action_id: StableId,
    pub worst_case_regret: ExactRational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionCertificate {
    pub schema_id: &'static str,
    pub manifest_digest: [u8; 32],
    pub decision_id: StableId,
    pub status: DecisionStatus,
    /// Present only for a unique minimax-regret action.
    pub minimax_regret_action: Option<StableId>,
    /// Present only when the same unique action is best in every hypothesis.
    pub stable_best_action: Option<StableId>,
    pub support_gaps: Vec<SupportGap>,
    pub hypothesis_regrets: Vec<HypothesisRegret>,
    pub action_regrets: Vec<ActionRegret>,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionError {
    EmptyActions,
    EmptyHypotheses,
    DuplicateAction,
    DuplicateHypothesis,
    DuplicateActionOutcome,
    UnknownAction,
    MissingActionOutcome,
    InvalidReferenceDigest,
    ZeroDenominator,
    ArithmeticOverflow,
    ReceiptMismatch,
}

impl DecisionManifest {
    pub fn validate(&self) -> Result<(), DecisionError> {
        if self.actions.is_empty() {
            return Err(DecisionError::EmptyActions);
        }
        if self.hypotheses.is_empty() {
            return Err(DecisionError::EmptyHypotheses);
        }

        let action_ids = self.actions.iter().cloned().collect::<BTreeSet<_>>();
        if action_ids.len() != self.actions.len() {
            return Err(DecisionError::DuplicateAction);
        }

        let mut hypothesis_ids = BTreeSet::new();
        for hypothesis in &self.hypotheses {
            if !hypothesis_ids.insert(hypothesis.hypothesis_id.clone()) {
                return Err(DecisionError::DuplicateHypothesis);
            }
            if hypothesis.reference_manifest_digest == [0; 32] {
                return Err(DecisionError::InvalidReferenceDigest);
            }

            let mut outcome_ids = BTreeSet::new();
            for outcome in &hypothesis.outcomes {
                if !action_ids.contains(&outcome.action_id) {
                    return Err(DecisionError::UnknownAction);
                }
                if !outcome_ids.insert(outcome.action_id.clone()) {
                    return Err(DecisionError::DuplicateActionOutcome);
                }
                if let OutcomeSupport::Supported { utility, .. } = &outcome.support
                    && utility.denominator == 0
                {
                    return Err(DecisionError::ZeroDenominator);
                }
            }

            if outcome_ids != action_ids {
                return Err(DecisionError::MissingActionOutcome);
            }
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, DecisionError> {
        self.validate()?;
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.decision_id);
        push_id(&mut out, &self.version);

        let mut actions = self.actions.iter().collect::<Vec<_>>();
        actions.sort();
        push_len(&mut out, actions.len());
        for action in actions {
            push_id(&mut out, action);
        }

        let mut hypotheses = self.hypotheses.iter().collect::<Vec<_>>();
        hypotheses.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
        push_len(&mut out, hypotheses.len());
        for hypothesis in hypotheses {
            push_id(&mut out, &hypothesis.hypothesis_id);
            out.extend_from_slice(&hypothesis.reference_manifest_digest);
            let mut outcomes = hypothesis.outcomes.iter().collect::<Vec<_>>();
            outcomes.sort_by(|a, b| a.action_id.cmp(&b.action_id));
            push_len(&mut out, outcomes.len());
            for outcome in outcomes {
                push_id(&mut out, &outcome.action_id);
                match &outcome.support {
                    OutcomeSupport::Supported {
                        utility,
                        evidence_digest,
                    } => {
                        out.push(1);
                        let normalized =
                            ExactRational::new(utility.numerator, utility.denominator)?;
                        push_rational(&mut out, &normalized);
                        out.extend_from_slice(evidence_digest);
                    }
                    OutcomeSupport::Unsupported { reason_id } => {
                        out.push(2);
                        push_id(&mut out, reason_id);
                    }
                    OutcomeSupport::NotIdentified { evidence_ref } => {
                        out.push(3);
                        push_id(&mut out, evidence_ref);
                    }
                    OutcomeSupport::Incomplete { reason_id } => {
                        out.push(4);
                        push_id(&mut out, reason_id);
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], DecisionError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(&self) -> Result<DecisionCertificate, DecisionError> {
        self.validate()?;
        let manifest_digest = self.digest()?;
        let mut gaps = Vec::new();

        for hypothesis in &self.hypotheses {
            for outcome in &hypothesis.outcomes {
                let gap = match &outcome.support {
                    OutcomeSupport::Supported { .. } => None,
                    OutcomeSupport::Unsupported { reason_id } => Some(SupportGap {
                        hypothesis_id: hypothesis.hypothesis_id.clone(),
                        action_id: outcome.action_id.clone(),
                        kind: SupportGapKind::Unsupported,
                        reason_id: reason_id.clone(),
                    }),
                    OutcomeSupport::NotIdentified { evidence_ref } => Some(SupportGap {
                        hypothesis_id: hypothesis.hypothesis_id.clone(),
                        action_id: outcome.action_id.clone(),
                        kind: SupportGapKind::NotIdentified,
                        reason_id: evidence_ref.clone(),
                    }),
                    OutcomeSupport::Incomplete { reason_id } => Some(SupportGap {
                        hypothesis_id: hypothesis.hypothesis_id.clone(),
                        action_id: outcome.action_id.clone(),
                        kind: SupportGapKind::Incomplete,
                        reason_id: reason_id.clone(),
                    }),
                };
                if let Some(gap) = gap {
                    gaps.push(gap);
                }
            }
        }
        gaps.sort_by(|a, b| {
            (&a.hypothesis_id, &a.action_id).cmp(&(&b.hypothesis_id, &b.action_id))
        });

        if !gaps.is_empty() {
            let status = if gaps.iter().any(|gap| {
                matches!(gap.kind, SupportGapKind::NotIdentified | SupportGapKind::Incomplete)
            }) {
                DecisionStatus::Incomplete
            } else {
                DecisionStatus::Unsupported
            };
            return Ok(finish_certificate(
                manifest_digest,
                self.decision_id.clone(),
                status,
                None,
                None,
                gaps,
                Vec::new(),
                Vec::new(),
            ));
        }

        let mut hypotheses = self.hypotheses.iter().collect::<Vec<_>>();
        hypotheses.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
        let mut hypothesis_regrets = Vec::with_capacity(hypotheses.len());
        let mut action_max_regrets = self
            .actions
            .iter()
            .map(|action| (action.clone(), ExactRational::new(0, 1).expect("valid zero")))
            .collect::<Vec<_>>();

        for hypothesis in &hypotheses {
            let mut utilities = hypothesis
                .outcomes
                .iter()
                .map(|outcome| match &outcome.support {
                    OutcomeSupport::Supported { utility, .. } => Ok((
                        outcome.action_id.clone(),
                        ExactRational::new(utility.numerator, utility.denominator)?,
                    ))
                    _ => Err(DecisionError::MissingActionOutcome),
                })
                .collect::<Result<Vec<_>, _>>()?;
            utilities.sort_by(|a, b| a.0.cmp(&b.0));

            let mut best_utility = utilities[0].1.clone();
            for (_, utility) in utilities.iter().skip(1) {
                if utility.checked_cmp(&best_utility)? == Ordering::Greater {
                    best_utility = utility.clone();
                }
            }

            let mut best_actions = Vec::new();
            let mut regrets = Vec::with_capacity(utilities.len());
            for (action_id, utility) in &utilities {
                if utility.checked_cmp(&best_utility)? == Ordering::Equal {
                    best_actions.push(action_id.clone());
                }
                let regret = best_utility.checked_sub(utility)?;
                regrets.push((action_id.clone(), regret.clone()));
                let entry = action_max_regrets
                    .iter_mut()
                    .find(|(candidate, _)| candidate == action_id)
                    .ok_or(DecisionError::UnknownAction)?;
                if regret.checked_cmp(&entry.1)? == Ordering::Greater {
                    entry.1 = regret;
                }
            }
            hypothesis_regrets.push(HypothesisRegret {
                hypothesis_id: hypothesis.hypothesis_id.clone(),
                best_utility,
                best_actions,
                regrets,
            });
        }

        action_max_regrets.sort_by(|a, b| a.0.cmp(&b.0));
        let mut minimax_regret = action_max_regrets[0].1.clone();
        for (_, regret) in action_max_regrets.iter().skip(1) {
            if regret.checked_cmp(&minimax_regret)? == Ordering::Less {
                minimax_regret = regret.clone();
            }
        }

        let mut minimax_actions = Vec::new();
        for (action, regret) in &action_max_regrets {
            if regret.checked_cmp(&minimax_regret)? == Ordering::Equal {
                minimax_actions.push(action.clone());
            }
        }

        let first_best = &hypothesis_regrets[0].best_actions;
        let stable = first_best.len() == 1
            && hypothesis_regrets
                .iter()
                .all(|result| result.best_actions == *first_best);
        let (status, minimax_action, stable_best_action) = if minimax_actions.len() > 1 {
            (DecisionStatus::NoRobustDecision, None, None)
        } else if stable {
            (
                DecisionStatus::DecisionStable,
                Some(minimax_actions[0].clone()),
                Some(first_best[0].clone()),
            )
        } else {
            (
                DecisionStatus::DecisionSensitive,
                Some(minimax_actions[0].clone()),
                None,
            )
        };

        Ok(finish_certificate(
            manifest_digest,
            self.decision_id.clone(),
            status,
            minimax_action,
            stable_best_action,
            Vec::new(),
            hypothesis_regrets,
            action_max_regrets
                .into_iter()
                .map(|(action_id, worst_case_regret)| ActionRegret {
                    action_id,
                    worst_case_regret,
                })
                .collect(),
        ))
    }
}

fn finish_certificate(
    manifest_digest: [u8; 32],
    decision_id: StableId,
    status: DecisionStatus,
    minimax_regret_action: Option<StableId>,
    stable_best_action: Option<StableId>,
    support_gaps: Vec<SupportGap>,
    hypothesis_regrets: Vec<HypothesisRegret>,
    action_regrets: Vec<ActionRegret>,
) -> DecisionCertificate {
    let mut certificate = DecisionCertificate {
        schema_id: "symtropy.road-decision-closure.certificate.v0.1",
        manifest_digest,
        decision_id,
        status,
        minimax_regret_action,
        stable_best_action,
        support_gaps,
        hypothesis_regrets,
        action_regrets,
        receipt_digest: [0; 32],
    };
    certificate.receipt_digest = *blake3::hash(&certificate_bytes(&certificate)).as_bytes();
    certificate
}

fn certificate_bytes(certificate: &DecisionCertificate) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-decision-closure.certificate.v0.1\0");
    push_bytes(&mut out, certificate.schema_id.as_bytes());
    out.extend_from_slice(&certificate.manifest_digest);
    push_id(&mut out, &certificate.decision_id);
    out.push(certificate.status as u8);
    push_option_id(&mut out, certificate.minimax_regret_action.as_ref());
    push_option_id(&mut out, certificate.stable_best_action.as_ref());

    push_len(&mut out, certificate.support_gaps.len());
    for gap in &certificate.support_gaps {
        push_id(&mut out, &gap.hypothesis_id);
        push_id(&mut out, &gap.action_id);
        out.push(gap.kind as u8);
        push_id(&mut out, &gap.reason_id);
    }

    push_len(&mut out, certificate.hypothesis_regrets.len());
    for hypothesis in &certificate.hypothesis_regrets {
        push_id(&mut out, &hypothesis.hypothesis_id);
        push_rational(&mut out, &hypothesis.best_utility);
        push_len(&mut out, hypothesis.best_actions.len());
        for action in &hypothesis.best_actions {
            push_id(&mut out, action);
        }
        push_len(&mut out, hypothesis.regrets.len());
        for (action, regret) in &hypothesis.regrets {
            push_id(&mut out, action);
            push_rational(&mut out, regret);
        }
    }

    push_len(&mut out, certificate.action_regrets.len());
    for action in &certificate.action_regrets {
        push_id(&mut out, &action.action_id);
        push_rational(&mut out, &action.worst_case_regret);
    }
    out
}

impl DecisionCertificate {
    /// Unkeyed checksum only. This is not authentication.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&certificate_bytes(self)).as_bytes() == self.receipt_digest
    }

    /// Recompute the complete decision certificate from the frozen manifest.
    pub fn verify_against(&self, manifest: &DecisionManifest) -> Result<(), DecisionError> {
        let recomputed = manifest.evaluate()?;
        if recomputed == *self {
            Ok(())
        } else {
            Err(DecisionError::ReceiptMismatch)
        }
    }
}

fn push_rational(out: &mut Vec<u8>, value: &ExactRational) {
    out.extend_from_slice(&value.numerator.to_le_bytes());
    out.extend_from_slice(&value.denominator.to_le_bytes());
}

fn push_option_id(out: &mut Vec<u8>, id: Option<&StableId>) {
    match id {
        Some(value) => {
            out.push(1);
            push_id(out, value);
        }
        None => out.push(0),
    }
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let length = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).expect("schema bytes length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let length = u64::try_from(length).expect("fixture collection length must fit u64");
    out.extend_from_slice(&length.to_le_bytes());
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::new(value).expect("fixture ID")
    }

    fn supported(action: &str, utility: i128) -> HypothesisActionOutcome {
        HypothesisActionOutcome {
            action_id: id(action),
            support: OutcomeSupport::Supported {
                utility: ExactRational::new(utility, 1).unwrap(),
                evidence_digest: *blake3::hash(action.as_bytes()).as_bytes(),
            },
        }
    }

    fn hypothesis(
        name: &str,
        a: i128,
        b: i128,
    ) -> DecisionHypothesis {
        DecisionHypothesis {
            hypothesis_id: id(name),
            reference_manifest_digest: *blake3::hash(name.as_bytes()).as_bytes(),
            outcomes: vec![supported("action-a", a), supported("action-b", b)],
        }
    }

    fn manifest(hypotheses: Vec<DecisionHypothesis>) -> DecisionManifest {
        DecisionManifest {
            decision_id: id("pedestrian-crossing-decision-01"),
            version: id("v0.1"),
            actions: vec![id("action-a"), id("action-b")],
            hypotheses,
        }
    }

    #[test]
    fn same_best_action_across_hypotheses_is_decision_stable() {
        let manifest = manifest(vec![
            hypothesis("dependence-model-1", 10, 2),
            hypothesis("dependence-model-2", 8, 4),
        ]);
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, DecisionStatus::DecisionStable);
        assert_eq!(certificate.minimax_regret_action, Some(id("action-a")));
        assert_eq!(certificate.stable_best_action, Some(id("action-a")));
        assert_eq!(
            certificate.action_regrets,
            vec![
                ActionRegret {
                    action_id: id("action-a"),
                    worst_case_regret: ExactRational::new(0, 1).unwrap(),
                },
                ActionRegret {
                    action_id: id("action-b"),
                    worst_case_regret: ExactRational::new(8, 1).unwrap(),
                }
            ]
        );
    }

    #[test]
    fn ranking_reversal_with_unique_minimax_regret_is_sensitive_not_stable() {
        let manifest = manifest(vec![
            hypothesis("reference-1", 10, 8),
            hypothesis("reference-2", 0, 6),
        ]);
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, DecisionStatus::DecisionSensitive);
        assert_eq!(certificate.minimax_regret_action, Some(id("action-b")));
        assert_eq!(certificate.stable_best_action, None);
        assert_eq!(
            certificate.action_regrets[0].worst_case_regret,
            ExactRational::new(6, 1).unwrap()
        );
        assert_eq!(
            certificate.action_regrets[1].worst_case_regret,
            ExactRational::new(2, 1).unwrap()
        );
    }

    #[test]
    fn symmetric_ranking_reversal_has_no_unique_minimax_action() {
        let manifest = manifest(vec![
            hypothesis("reference-1", 10, 0),
            hypothesis("reference-2", 0, 10),
        ]);
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(certificate.status, DecisionStatus::NoRobustDecision);
        assert_eq!(certificate.minimax_regret_action, None);
        assert_eq!(certificate.stable_best_action, None);
    }

    #[test]
    fn unsupported_hypothesis_action_blocks_global_decision() {
        let mut h = hypothesis("reference-1", 10, 2);
        h.outcomes[1].support = OutcomeSupport::Unsupported {
            reason_id: id("unsupported-action-region"),
        };
        let certificate = manifest(vec![h, hypothesis("reference-2", 8, 4)])
            .evaluate()
            .unwrap();
        assert_eq!(certificate.status, DecisionStatus::Unsupported);
        assert_eq!(certificate.minimax_regret_action, None);
        assert_eq!(certificate.support_gaps.len(), 1);
        assert_eq!(certificate.support_gaps[0].kind, SupportGapKind::Unsupported);
    }

    #[test]
    fn unidentified_or_incomplete_hypothesis_blocks_global_decision() {
        let mut h = hypothesis("reference-1", 10, 2);
        h.outcomes[1].support = OutcomeSupport::NotIdentified {
            evidence_ref: id("counterfactual-support-not-established"),
        };
        let certificate = manifest(vec![h]).evaluate().unwrap();
        assert_eq!(certificate.status, DecisionStatus::Incomplete);
        assert_eq!(certificate.minimax_regret_action, None);
    }

    #[test]
    fn no_implicit_hypothesis_probability_weights_exist() {
        let manifest = manifest(vec![
            hypothesis("model-a", 10, 2),
            hypothesis("model-b", 0, 8),
            hypothesis("model-c", 4, 5),
        ]);
        let certificate = manifest.evaluate().unwrap();
        assert_eq!(
            certificate.hypothesis_regrets.len(),
            3,
            "every admitted reference hypothesis must contribute"
        );
    }

    #[test]
    fn declaration_order_does_not_change_manifest_or_certificate() {
        let first = manifest(vec![
            hypothesis("model-a", 10, 2),
            hypothesis("model-b", 8, 4),
        ]);
        let mut second = first.clone();
        second.actions.reverse();
        second.hypotheses.reverse();
        for hypothesis in &mut second.hypotheses {
            hypothesis.outcomes.reverse();
        }
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
        assert_eq!(first.evaluate().unwrap(), second.evaluate().unwrap());
    }

    #[test]
    fn missing_duplicate_and_unknown_action_outcomes_reject() {
        let mut missing = hypothesis("reference-1", 10, 2);
        missing.outcomes.pop();
        assert_eq!(
            manifest(vec![missing]).validate(),
            Err(DecisionError::MissingActionOutcome)
        );

        let mut duplicate = hypothesis("reference-1", 10, 2);
        duplicate.outcomes.push(duplicate.outcomes[0].clone());
        assert_eq!(
            manifest(vec![duplicate]).validate(),
            Err(DecisionError::DuplicateActionOutcome)
        );

        let mut unknown = hypothesis("reference-1", 10, 2);
        unknown.outcomes[0].action_id = id("action-unknown");
        assert_eq!(
            manifest(vec![unknown]).validate(),
            Err(DecisionError::UnknownAction)
        );
    }

    #[test]
    fn receipt_tampering_and_manifest_change_are_detected() {
        let original = manifest(vec![
            hypothesis("reference-1", 10, 2),
            hypothesis("reference-2", 8, 4),
        ]);
        let certificate = original.evaluate().unwrap();
        assert!(certificate.integrity_check());
        assert_eq!(certificate.verify_against(&original), Ok(()));

        let mut changed = original;
        changed.hypotheses[0].outcomes[0] = supported("action-a", 7);
        assert_eq!(
            certificate.verify_against(&changed),
            Err(DecisionError::ReceiptMismatch)
        );
    }

    #[test]
    fn equivalent_rational_representations_have_identical_manifest_identity() {
        let first = manifest(vec![hypothesis("reference-1", 3, 2)]);
        let mut second = first.clone();
        second.hypotheses[0].outcomes[0].support = OutcomeSupport::Supported {
            utility: ExactRational {
                numerator: 30,
                denominator: 10,
            },
            evidence_digest: *blake3::hash(b"action-a").as_bytes(),
        };
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
    }

    #[test]
    fn missing_reference_manifest_digest_is_rejected() {
        let mut hypothesis = hypothesis("reference-1", 3, 2);
        hypothesis.reference_manifest_digest = [0; 32];
        assert_eq!(
            manifest(vec![hypothesis]).validate(),
            Err(DecisionError::InvalidReferenceDigest)
        );
    }

    #[test]
    fn exact_rational_arithmetic_normalizes_and_detects_overflow() {
        assert_eq!(
            ExactRational::new(6, 8).unwrap(),
            ExactRational::new(3, 4).unwrap()
        );
        assert_eq!(
            ExactRational::new(1, 0),
            Err(DecisionError::ZeroDenominator)
        );
        assert_eq!(
            ExactRational::new(i128::MAX, 1)
                .unwrap()
                .checked_sub(&ExactRational::new(-1, 1).unwrap()),
            Err(DecisionError::ArithmeticOverflow)
        );
    }
}
