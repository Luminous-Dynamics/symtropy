//! Exact finite-state temporal dependence reference.
//!
//! This analytic kernel distinguishes process innovations from process states.
//! It is not an empirical human-behavior model or a general stochastic solver.

use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;

const DOMAIN: &[u8] = b"symtropy.road-temporal-dependence.v0.1\0";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum InnovationCoupling {
    SharedInvariant {
        equivalence_group: StableId,
        justification: StableId,
    },
    BranchSpecific {
        branch: StableId,
    },
    NotIdentified {
        evidence_ref: StableId,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TransitionRule {
    /// X[t+1] = X[t] * F[t+1].
    MultiplyPriorByInnovation,
    /// X[t+1] = F[t+1]; this is the independent-per-tick negative control.
    ReplaceStateWithInnovation,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TemporalJointState {
    pub initial_state: i64,
    pub transition_innovation: i64,
    pub weight: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalProcessManifest {
    pub process_id: StableId,
    pub version: StableId,
    pub mechanism_id: StableId,
    pub source_profile_digest: [u8; 32],
    pub time_unit: StableId,
    pub initial_tick: u64,
    pub next_tick: u64,
    pub initial_state_id: StableId,
    pub innovation_id: StableId,
    pub next_state_id: StableId,
    pub intervention_id: StableId,
    pub initial_coupling: InnovationCoupling,
    pub innovation_coupling: InnovationCoupling,
    pub transition_rule: TransitionRule,
    pub states: Vec<TemporalJointState>,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalIntervention {
    pub branch_id: StableId,
    /// A declared additive intervention on the next-state structural equation.
    pub next_state_offset: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalError {
    EmptyStateTable,
    InvalidTimeOrder,
    NonContiguousTick,
    EmptyTimeUnit,
    ZeroStateWeight,
    DuplicateJointState,
    WeightOverflow,
    InvalidBinaryState,
    InnovationsNotIndependentOfInitialState,
    NotIdentified,
    WrongBranchScope,
    DuplicateProcessIdentity,
    ArithmeticOverflow,
    ReceiptMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRational {
    pub numerator: i128,
    pub denominator: u128,
}

impl ExactRational {
    fn new(numerator: i128, denominator: u128) -> Result<Self, TemporalError> {
        if denominator == 0 {
            return Err(TemporalError::ArithmeticOverflow);
        }
        if numerator == 0 {
            return Ok(Self {
                numerator: 0,
                denominator: 1,
            });
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        let signed_divisor =
            i128::try_from(divisor).map_err(|_| TemporalError::ArithmeticOverflow)?;
        Ok(Self {
            numerator: numerator / signed_divisor,
            denominator: denominator / divisor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalReceipt {
    pub schema_id: &'static str,
    pub manifest_digest: [u8; 32],
    pub branch_id: StableId,
    pub path_index: usize,
    pub path_weight: u64,
    pub initial_tick: u64,
    pub next_tick: u64,
    pub initial_state: i64,
    pub transition_innovation: i64,
    pub base_next_state: i64,
    pub intervention_offset: i64,
    pub next_state: i64,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalEvaluation {
    pub branch_id: StableId,
    pub total_weight: u64,
    pub receipts: Vec<TemporalReceipt>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalAxis {
    Initial,
    Next,
}

impl TemporalProcessManifest {
    /// Validate the complete finite joint law and return its total integer mass.
    ///
    /// V0.1 explicitly assumes initial state and transition innovation are
    /// independent; the complete path table is checked against that assumption.
    pub fn validate(&self) -> Result<u64, TemporalError> {
        if self.states.is_empty() {
            return Err(TemporalError::EmptyStateTable);
        }
        if self.time_unit.as_str().is_empty() {
            return Err(TemporalError::EmptyTimeUnit);
        }
        if self.next_tick <= self.initial_tick {
            return Err(TemporalError::InvalidTimeOrder);
        }
        if self.initial_tick.checked_add(1) != Some(self.next_tick) {
            return Err(TemporalError::NonContiguousTick);
        }

        let mut paths = BTreeSet::new();
        let mut total = 0u64;
        let mut initial_marginal = BTreeMap::<i64, u64>::new();
        let mut innovation_marginal = BTreeMap::<i64, u64>::new();

        for state in &self.states {
            if state.weight == 0 {
                return Err(TemporalError::ZeroStateWeight);
            }
            if !matches!(state.initial_state, -1 | 1)
                || !matches!(state.transition_innovation, -1 | 1)
            {
                return Err(TemporalError::InvalidBinaryState);
            }
            if !paths.insert((state.initial_state, state.transition_innovation)) {
                return Err(TemporalError::DuplicateJointState);
            }

            total = total
                .checked_add(state.weight)
                .ok_or(TemporalError::WeightOverflow)?;

            let initial_mass = initial_marginal.entry(state.initial_state).or_default();
            *initial_mass = initial_mass
                .checked_add(state.weight)
                .ok_or(TemporalError::WeightOverflow)?;

            let innovation_mass = innovation_marginal
                .entry(state.transition_innovation)
                .or_default();
            *innovation_mass = innovation_mass
                .checked_add(state.weight)
                .ok_or(TemporalError::WeightOverflow)?;
        }

        if total == 0 {
            return Err(TemporalError::WeightOverflow);
        }

        for state in &self.states {
            let initial_mass = u128::from(initial_marginal[&state.initial_state]);
            let innovation_mass =
                u128::from(innovation_marginal[&state.transition_innovation]);
            let left = u128::from(state.weight)
                .checked_mul(u128::from(total))
                .ok_or(TemporalError::ArithmeticOverflow)?;
            let right = initial_mass
                .checked_mul(innovation_mass)
                .ok_or(TemporalError::ArithmeticOverflow)?;
            if left != right {
                return Err(TemporalError::InnovationsNotIndependentOfInitialState);
            }
        }

        Ok(total)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, TemporalError> {
        self.validate()?;

        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.process_id);
        push_id(&mut out, &self.version);
        push_id(&mut out, &self.mechanism_id);
        out.extend_from_slice(&self.source_profile_digest);
        push_id(&mut out, &self.time_unit);
        out.extend_from_slice(&self.initial_tick.to_le_bytes());
        out.extend_from_slice(&self.next_tick.to_le_bytes());
        push_id(&mut out, &self.initial_state_id);
        push_id(&mut out, &self.innovation_id);
        push_id(&mut out, &self.next_state_id);
        push_id(&mut out, &self.intervention_id);
        push_id(&mut out, &self.evidence_ref);
        push_coupling(&mut out, &self.initial_coupling);
        push_coupling(&mut out, &self.innovation_coupling);
        out.push(match self.transition_rule {
            TransitionRule::MultiplyPriorByInnovation => 1,
            TransitionRule::ReplaceStateWithInnovation => 2,
        });

        let mut states = self.states.iter().collect::<Vec<_>>();
        states.sort_by_key(|state| (state.initial_state, state.transition_innovation));
        push_len(&mut out, states.len());
        for state in states {
            out.extend_from_slice(&state.initial_state.to_le_bytes());
            out.extend_from_slice(&state.transition_innovation.to_le_bytes());
            out.extend_from_slice(&state.weight.to_le_bytes());
        }
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], TemporalError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(
        &self,
        intervention: &TemporalIntervention,
    ) -> Result<TemporalEvaluation, TemporalError> {
        let total_weight = self.validate()?;
        validate_coupling(&self.initial_coupling, &intervention.branch_id)?;
        validate_coupling(&self.innovation_coupling, &intervention.branch_id)?;
        let manifest_digest = self.digest()?;

        let mut states = self.states.iter().collect::<Vec<_>>();
        states.sort_by_key(|state| (state.initial_state, state.transition_innovation));

        let mut receipts = Vec::with_capacity(states.len());
        for (path_index, state) in states.into_iter().enumerate() {
            let base_next_state = match self.transition_rule {
                TransitionRule::MultiplyPriorByInnovation => state
                    .initial_state
                    .checked_mul(state.transition_innovation)
                    .ok_or(TemporalError::ArithmeticOverflow)?,
                TransitionRule::ReplaceStateWithInnovation => state.transition_innovation,
            };
            let next_state = base_next_state
                .checked_add(intervention.next_state_offset)
                .ok_or(TemporalError::ArithmeticOverflow)?;

            let mut receipt = TemporalReceipt {
                schema_id: "symtropy.road-temporal-dependence.receipt.v0.1",
                manifest_digest,
                branch_id: intervention.branch_id.clone(),
                path_index,
                path_weight: state.weight,
                initial_tick: self.initial_tick,
                next_tick: self.next_tick,
                initial_state: state.initial_state,
                transition_innovation: state.transition_innovation,
                base_next_state,
                intervention_offset: intervention.next_state_offset,
                next_state,
                receipt_digest: [0; 32],
            };
            receipt.receipt_digest = *blake3::hash(&receipt_bytes(&receipt)).as_bytes();
            receipts.push(receipt);
        }

        Ok(TemporalEvaluation {
            branch_id: intervention.branch_id.clone(),
            total_weight,
            receipts,
        })
    }
}

impl TemporalEvaluation {
    pub fn mean(&self, axis: TemporalAxis) -> Result<ExactRational, TemporalError> {
        let mut weighted_sum = 0i128;
        for receipt in &self.receipts {
            let value = match axis {
                TemporalAxis::Initial => receipt.initial_state,
                TemporalAxis::Next => receipt.next_state,
            };
            let contribution = i128::from(value)
                .checked_mul(i128::from(receipt.path_weight))
                .ok_or(TemporalError::ArithmeticOverflow)?;
            weighted_sum = weighted_sum
                .checked_add(contribution)
                .ok_or(TemporalError::ArithmeticOverflow)?;
        }
        ExactRational::new(weighted_sum, u128::from(self.total_weight))
    }

    pub fn variance(&self, axis: TemporalAxis) -> Result<ExactRational, TemporalError> {
        self.covariance(axis, axis)
    }

    pub fn covariance(
        &self,
        left_axis: TemporalAxis,
        right_axis: TemporalAxis,
    ) -> Result<ExactRational, TemporalError> {
        let mut sum_left = 0i128;
        let mut sum_right = 0i128;
        let mut sum_product = 0i128;

        for receipt in &self.receipts {
            let left = i128::from(match left_axis {
                TemporalAxis::Initial => receipt.initial_state,
                TemporalAxis::Next => receipt.next_state,
            });
            let right = i128::from(match right_axis {
                TemporalAxis::Initial => receipt.initial_state,
                TemporalAxis::Next => receipt.next_state,
            });
            let weight = i128::from(receipt.path_weight);

            sum_left = sum_left
                .checked_add(
                    left.checked_mul(weight)
                        .ok_or(TemporalError::ArithmeticOverflow)?,
                )
                .ok_or(TemporalError::ArithmeticOverflow)?;
            sum_right = sum_right
                .checked_add(
                    right
                        .checked_mul(weight)
                        .ok_or(TemporalError::ArithmeticOverflow)?,
                )
                .ok_or(TemporalError::ArithmeticOverflow)?;
            sum_product = sum_product
                .checked_add(
                    left.checked_mul(right)
                        .and_then(|value| value.checked_mul(weight))
                        .ok_or(TemporalError::ArithmeticOverflow)?,
                )
                .ok_or(TemporalError::ArithmeticOverflow)?;
        }

        let total = i128::from(self.total_weight);
        let numerator = sum_product
            .checked_mul(total)
            .and_then(|value| {
                sum_left
                    .checked_mul(sum_right)
                    .and_then(|means| value.checked_sub(means))
            })
            .ok_or(TemporalError::ArithmeticOverflow)?;
        let denominator = u128::from(self.total_weight)
            .checked_mul(u128::from(self.total_weight))
            .ok_or(TemporalError::ArithmeticOverflow)?;
        ExactRational::new(numerator, denominator)
    }

    pub fn marginal_probabilities(
        &self,
        axis: TemporalAxis,
    ) -> Result<BTreeMap<i64, ExactRational>, TemporalError> {
        let mut counts = BTreeMap::<i64, u64>::new();
        for receipt in &self.receipts {
            let value = match axis {
                TemporalAxis::Initial => receipt.initial_state,
                TemporalAxis::Next => receipt.next_state,
            };
            let count = counts.entry(value).or_default();
            *count = count
                .checked_add(receipt.path_weight)
                .ok_or(TemporalError::WeightOverflow)?;
        }
        counts
            .into_iter()
            .map(|(value, mass)| {
                Ok((
                    value,
                    ExactRational::new(
                        i128::from(mass),
                        u128::from(self.total_weight),
                    )?,
                ))
            })
            .collect()
    }
}

impl TemporalReceipt {
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&receipt_bytes(self)).as_bytes() == self.receipt_digest
    }

    pub fn verify_against(
        &self,
        manifest: &TemporalProcessManifest,
        intervention: &TemporalIntervention,
    ) -> Result<(), TemporalError> {
        let evaluation = manifest.evaluate(intervention)?;
        if evaluation.receipts.iter().any(|candidate| candidate == self) {
            Ok(())
        } else {
            Err(TemporalError::ReceiptMismatch)
        }
    }
}

fn validate_coupling(
    coupling: &InnovationCoupling,
    branch: &StableId,
) -> Result<(), TemporalError> {
    match coupling {
        InnovationCoupling::SharedInvariant { .. } => Ok(()),
        InnovationCoupling::BranchSpecific { branch: scope } if scope == branch => Ok(()),
        InnovationCoupling::BranchSpecific { .. } => Err(TemporalError::WrongBranchScope),
        InnovationCoupling::NotIdentified { .. } => Err(TemporalError::NotIdentified),
    }
}

fn push_coupling(out: &mut Vec<u8>, coupling: &InnovationCoupling) {
    match coupling {
        InnovationCoupling::SharedInvariant {
            equivalence_group,
            justification,
        } => {
            out.push(1);
            push_id(out, equivalence_group);
            push_id(out, justification);
        }
        InnovationCoupling::BranchSpecific { branch } => {
            out.push(2);
            push_id(out, branch);
        }
        InnovationCoupling::NotIdentified { evidence_ref } => {
            out.push(3);
            push_id(out, evidence_ref);
        }
    }
}

fn receipt_bytes(receipt: &TemporalReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-temporal-dependence.receipt.v0.1\0");
    push_bytes(&mut out, receipt.schema_id.as_bytes());
    out.extend_from_slice(&receipt.manifest_digest);
    push_id(&mut out, &receipt.branch_id);
    push_len(&mut out, receipt.path_index);
    out.extend_from_slice(&receipt.path_weight.to_le_bytes());
    out.extend_from_slice(&receipt.initial_tick.to_le_bytes());
    out.extend_from_slice(&receipt.next_tick.to_le_bytes());
    out.extend_from_slice(&receipt.initial_state.to_le_bytes());
    out.extend_from_slice(&receipt.transition_innovation.to_le_bytes());
    out.extend_from_slice(&receipt.base_next_state.to_le_bytes());
    out.extend_from_slice(&receipt.intervention_offset.to_le_bytes());
    out.extend_from_slice(&receipt.next_state.to_le_bytes());
    out
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let len = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u64::try_from(bytes.len()).expect("fixture bytes must fit u64");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_len(out: &mut Vec<u8>, len: usize) {
    let len = u64::try_from(len).expect("fixture length must fit u64");
    out.extend_from_slice(&len.to_le_bytes());
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

    fn coupling() -> InnovationCoupling {
        InnovationCoupling::SharedInvariant {
            equivalence_group: id("road-process-family"),
            justification: id("scm:shared-process-innovations"),
        }
    }

    fn markov_fixture(rule: TransitionRule) -> TemporalProcessManifest {
        let states = match rule {
            TransitionRule::MultiplyPriorByInnovation => vec![
                TemporalJointState {
                    initial_state: -1,
                    transition_innovation: -1,
                    weight: 1,
                },
                TemporalJointState {
                    initial_state: -1,
                    transition_innovation: 1,
                    weight: 3,
                },
                TemporalJointState {
                    initial_state: 1,
                    transition_innovation: -1,
                    weight: 1,
                },
                TemporalJointState {
                    initial_state: 1,
                    transition_innovation: 1,
                    weight: 3,
                },
            ],
            TransitionRule::ReplaceStateWithInnovation => vec![
                TemporalJointState {
                    initial_state: -1,
                    transition_innovation: -1,
                    weight: 2,
                },
                TemporalJointState {
                    initial_state: -1,
                    transition_innovation: 1,
                    weight: 2,
                },
                TemporalJointState {
                    initial_state: 1,
                    transition_innovation: -1,
                    weight: 2,
                },
                TemporalJointState {
                    initial_state: 1,
                    transition_innovation: 1,
                    weight: 2,
                },
            ],
        };
        TemporalProcessManifest {
            process_id: id("pedestrian-collective-motion"),
            version: id("v0.1"),
            mechanism_id: id("binary-transition-v1"),
            source_profile_digest: *blake3::hash(b"binary-process-profile-v1").as_bytes(),
            time_unit: id("simulation-tick"),
            initial_tick: 0,
            next_tick: 1,
            initial_state_id: id("state-at-zero"),
            innovation_id: id("transition-innovation-at-one"),
            next_state_id: id("state-at-one"),
            intervention_id: id("ego-yield-offset"),
            initial_coupling: coupling(),
            innovation_coupling: coupling(),
            transition_rule: rule,
            states,
            evidence_ref: id("fixture:exact-finite-transition-law"),
        }
    }

    #[test]
    fn binary_markov_transition_has_exact_lag_one_covariance_one_half() {
        let evaluation = markov_fixture(TransitionRule::MultiplyPriorByInnovation)
            .evaluate(&TemporalIntervention {
                branch_id: id("continue"),
                next_state_offset: 0,
            })
            .unwrap();

        assert_eq!(evaluation.total_weight, 8);
        assert_eq!(
            evaluation.mean(TemporalAxis::Initial).unwrap(),
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
        assert_eq!(
            evaluation.mean(TemporalAxis::Next).unwrap(),
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
        assert_eq!(
            evaluation.variance(TemporalAxis::Initial).unwrap(),
            ExactRational {
                numerator: 1,
                denominator: 1
            }
        );
        assert_eq!(
            evaluation.variance(TemporalAxis::Next).unwrap(),
            ExactRational {
                numerator: 1,
                denominator: 1
            }
        );
        assert_eq!(
            evaluation
                .covariance(TemporalAxis::Initial, TemporalAxis::Next)
                .unwrap(),
            ExactRational {
                numerator: 1,
                denominator: 2
            }
        );
    }

    #[test]
    fn independent_per_tick_control_has_same_marginals_but_zero_lag_covariance() {
        let temporal = markov_fixture(TransitionRule::MultiplyPriorByInnovation)
            .evaluate(&TemporalIntervention {
                branch_id: id("continue"),
                next_state_offset: 0,
            })
            .unwrap();
        let independent = markov_fixture(TransitionRule::ReplaceStateWithInnovation)
            .evaluate(&TemporalIntervention {
                branch_id: id("continue"),
                next_state_offset: 0,
            })
            .unwrap();

        assert_eq!(
            temporal
                .marginal_probabilities(TemporalAxis::Initial)
                .unwrap(),
            independent
                .marginal_probabilities(TemporalAxis::Initial)
                .unwrap()
        );
        assert_eq!(
            temporal
                .marginal_probabilities(TemporalAxis::Next)
                .unwrap(),
            independent
                .marginal_probabilities(TemporalAxis::Next)
                .unwrap()
        );
        assert_eq!(
            temporal
                .covariance(TemporalAxis::Initial, TemporalAxis::Next)
                .unwrap(),
            ExactRational {
                numerator: 1,
                denominator: 2
            }
        );
        assert_eq!(
            independent
                .covariance(TemporalAxis::Initial, TemporalAxis::Next)
                .unwrap(),
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
    }

    #[test]
    fn intervention_recomputes_state_but_preserves_shared_innovations() {
        let manifest = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        let continue_intervention = TemporalIntervention {
            branch_id: id("continue"),
            next_state_offset: 0,
        };
        let yield_intervention = TemporalIntervention {
            branch_id: id("yield"),
            next_state_offset: 2,
        };
        let continued = manifest.evaluate(&continue_intervention).unwrap();
        let yielded = manifest.evaluate(&yield_intervention).unwrap();

        for (left, right) in continued.receipts.iter().zip(&yielded.receipts) {
            assert_eq!(left.initial_state, right.initial_state);
            assert_eq!(left.transition_innovation, right.transition_innovation);
            assert_eq!(left.base_next_state, right.base_next_state);
            assert_eq!(right.next_state - left.next_state, 2);
            assert!(left.integrity_check());
            assert!(right.integrity_check());
            assert_eq!(
                left.verify_against(&manifest, &continue_intervention),
                Ok(())
            );
            assert_eq!(
                right.verify_against(&manifest, &yield_intervention),
                Ok(())
            );
        }
    }

    #[test]
    fn declaration_order_does_not_change_manifest_digest_or_receipts() {
        let first = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        let mut second = first.clone();
        second.states.reverse();
        let a = TemporalIntervention {
            branch_id: id("continue"),
            next_state_offset: 0,
        };
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
        assert_eq!(
            first.evaluate(&a).unwrap(),
            second.evaluate(&a).unwrap()
        );
    }

    #[test]
    fn correlated_initial_and_innovation_inputs_are_rejected_by_declared_independence_contract() {
        let mut manifest = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        manifest.states = vec![
            TemporalJointState {
                initial_state: -1,
                transition_innovation: -1,
                weight: 4,
            },
            TemporalJointState {
                initial_state: 1,
                transition_innovation: 1,
                weight: 4,
            },
        ];
        assert_eq!(
            manifest.validate(),
            Err(TemporalError::InnovationsNotIndependentOfInitialState)
        );
    }

    #[test]
    fn unknown_innovation_cross_world_role_is_not_shared_by_default() {
        let mut manifest = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        manifest.innovation_coupling = InnovationCoupling::NotIdentified {
            evidence_ref: id("evidence:unknown-cross-world-role"),
        };
        assert_eq!(
            manifest.evaluate(&TemporalIntervention {
                branch_id: id("continue"),
                next_state_offset: 0,
            }),
            Err(TemporalError::NotIdentified)
        );
    }

    #[test]
    fn branch_specific_innovation_cannot_be_replayed_in_sibling_branch() {
        let mut manifest = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        manifest.innovation_coupling = InnovationCoupling::BranchSpecific {
            branch: id("continue"),
        };
        assert_eq!(
            manifest.evaluate(&TemporalIntervention {
                branch_id: id("yield"),
                next_state_offset: 0,
            }),
            Err(TemporalError::WrongBranchScope)
        );
    }

    #[test]
    fn duplicate_paths_zero_mass_and_bad_time_order_reject() {
        let mut duplicate = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        duplicate.states.push(duplicate.states[0].clone());
        assert_eq!(
            duplicate.validate(),
            Err(TemporalError::DuplicateJointState)
        );

        let mut zero = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        zero.states[0].weight = 0;
        assert_eq!(zero.validate(), Err(TemporalError::ZeroStateWeight));

        let mut unordered = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        unordered.next_tick = 0;
        assert_eq!(
            unordered.validate(),
            Err(TemporalError::InvalidTimeOrder)
        );
    }

    #[test]
    fn source_profile_and_transition_version_are_bound_to_manifest_identity() {
        let first = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        let mut changed_profile = first.clone();
        changed_profile.source_profile_digest[0] ^= 1;
        assert_ne!(first.digest().unwrap(), changed_profile.digest().unwrap());

        let mut changed_mechanism = first.clone();
        changed_mechanism.mechanism_id = id("binary-transition-v2");
        assert_ne!(first.digest().unwrap(), changed_mechanism.digest().unwrap());
    }

    #[test]
    fn receipt_tampering_and_intervention_mismatch_are_detected() {
        let manifest = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        let intervention = TemporalIntervention {
            branch_id: id("continue"),
            next_state_offset: 0,
        };
        let mut receipt = manifest.evaluate(&intervention).unwrap().receipts[0].clone();
        receipt.next_state ^= 1;
        assert!(!receipt.integrity_check());
        assert_eq!(
            receipt.verify_against(&manifest, &intervention),
            Err(TemporalError::ReceiptMismatch)
        );
    }

    #[test]
    fn joint_mass_overflow_is_rejected() {
        let mut manifest = markov_fixture(TransitionRule::MultiplyPriorByInnovation);
        manifest.states[0].weight = u64::MAX;
        manifest.states[1].weight = 1;
        assert_eq!(manifest.validate(), Err(TemporalError::WeightOverflow));
    }
}
