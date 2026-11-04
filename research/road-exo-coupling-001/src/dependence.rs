//! Exact finite-state dependence graph reference.
//!
//! This module is deliberately an analytic oracle, not a production random-field
//! sampler. A complete joint finite distribution defines dependence explicitly;
//! deterministic structural outputs reuse its shared latent factors.

use std::collections::{BTreeMap, BTreeSet};

use crate::StableId;

const DOMAIN: &[u8] = b"symtropy.road-dependence.finite-graph.v0.1\0";

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FactorScope {
    Scene,
    Group(StableId),
    Actor(StableId),
    DirectedDyad {
        initiator: StableId,
        recipient: StableId,
    },
    SpatialBasisCoefficient(StableId),
    TimeStep(u64),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FactorVariable {
    pub id: StableId,
    pub source_id: StableId,
    pub source_profile_digest: [u8; 32],
    pub units: StableId,
    pub scope: FactorScope,
    pub equivalence_group: StableId,
    pub invariance_justification: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct WeightedJointState {
    /// Values use the same variable IDs as the distribution manifest.
    pub values: Vec<(StableId, i64)>,
    /// Positive integer mass; probabilities are weight / sum(weights).
    pub weight: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointInnovationDistribution {
    pub states: Vec<WeightedJointState>,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum StructuralInput {
    Exogenous(StableId),
    Intervention(StableId),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LinearTerm {
    pub input: StructuralInput,
    pub coefficient: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LinearOutput {
    pub id: StableId,
    pub units: StableId,
    pub intercept: i64,
    /// Sum is evaluated in stable input-ID order, with checked arithmetic.
    pub terms: Vec<LinearTerm>,
    pub mechanism_id: StableId,
    pub evidence_ref: StableId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorGraphManifest {
    pub graph_id: StableId,
    pub version: StableId,
    pub variables: Vec<FactorVariable>,
    pub distribution: JointInnovationDistribution,
    pub outputs: Vec<LinearOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchIntervention {
    pub branch_id: StableId,
    pub values: Vec<(StableId, i64)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependenceError {
    EmptyVariables,
    EmptyStates,
    EmptyOutputs,
    DuplicateVariable,
    DuplicateOutput,
    DuplicateIntervention,
    DuplicateTerm,
    DuplicateStateVariable,
    DuplicateJointState,
    MissingStateVariable,
    UnknownStateVariable,
    UnknownOutputInput,
    UnknownIntervention,
    ZeroStateWeight,
    WeightOverflow,
    WeightTotalMismatch,
    WeightTotalZero,
    InvalidDirectedDyad,
    SameBranch,
    UnknownVariable,
    UnknownOutput,
    MissingInterventionValue,
    ArithmeticOverflow,
    ReceiptMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRational {
    pub numerator: i128,
    pub denominator: u128,
}

impl ExactRational {
    fn new(numerator: i128, denominator: u128) -> Result<Self, DependenceError> {
        if denominator == 0 {
            return Err(DependenceError::ArithmeticOverflow);
        }
        if numerator == 0 {
            return Ok(Self {
                numerator: 0,
                denominator: 1,
            });
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        let signed_divisor =
            i128::try_from(divisor).map_err(|_| DependenceError::ArithmeticOverflow)?;
        Ok(Self {
            numerator: numerator / signed_divisor,
            denominator: denominator / divisor,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependenceReceipt {
    pub schema_id: &'static str,
    pub manifest_digest: [u8; 32],
    pub branch_id: StableId,
    pub state_index: usize,
    pub state_weight: u64,
    pub latent_values: Vec<(StableId, i64)>,
    pub intervention_values: Vec<(StableId, i64)>,
    pub output_values: Vec<(StableId, i64)>,
    pub receipt_digest: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchEvaluation {
    pub branch_id: StableId,
    pub total_weight: u64,
    pub receipts: Vec<DependenceReceipt>,
}

impl BranchEvaluation {
    pub fn mean(&self, output_id: &StableId) -> Result<ExactRational, DependenceError> {
        let mut weighted_sum = 0i128;
        for receipt in &self.receipts {
            let value = output_value(receipt, output_id)?;
            let weighted_value = i128::from(value)
                .checked_mul(i128::from(receipt.state_weight))
                .ok_or(DependenceError::ArithmeticOverflow)?;
            weighted_sum = weighted_sum
                .checked_add(weighted_value)
                .ok_or(DependenceError::ArithmeticOverflow)?;
        }
        ExactRational::new(weighted_sum, u128::from(self.total_weight))
    }

    pub fn covariance(
        &self,
        left_output: &StableId,
        right_output: &StableId,
    ) -> Result<ExactRational, DependenceError> {
        let mut sum_left = 0i128;
        let mut sum_right = 0i128;
        let mut sum_product = 0i128;

        for receipt in &self.receipts {
            let weight = i128::from(receipt.state_weight);
            let left = i128::from(output_value(receipt, left_output)?);
            let right = i128::from(output_value(receipt, right_output)?);
            sum_left = sum_left
                .checked_add(
                    left.checked_mul(weight)
                        .ok_or(DependenceError::ArithmeticOverflow)?,
                )
                .ok_or(DependenceError::ArithmeticOverflow)?;
            sum_right = sum_right
                .checked_add(
                    right
                        .checked_mul(weight)
                        .ok_or(DependenceError::ArithmeticOverflow)?,
                )
                .ok_or(DependenceError::ArithmeticOverflow)?;
            sum_product = sum_product
                .checked_add(
                    left.checked_mul(right)
                        .and_then(|value| value.checked_mul(weight))
                        .ok_or(DependenceError::ArithmeticOverflow)?,
                )
                .ok_or(DependenceError::ArithmeticOverflow)?;
        }

        let total = i128::from(self.total_weight);
        let numerator = sum_product
            .checked_mul(total)
            .and_then(|value| {
                sum_left
                    .checked_mul(sum_right)
                    .and_then(|means| value.checked_sub(means))
            })
            .ok_or(DependenceError::ArithmeticOverflow)?;
        let denominator = u128::from(self.total_weight)
            .checked_mul(u128::from(self.total_weight))
            .ok_or(DependenceError::ArithmeticOverflow)?;
        ExactRational::new(numerator, denominator)
    }

    pub fn marginal_probabilities(
        &self,
        output_id: &StableId,
    ) -> Result<BTreeMap<i64, ExactRational>, DependenceError> {
        self.marginal_counts(output_id)?
            .into_iter()
            .map(|(value, weight)| {
                Ok((
                    value,
                    ExactRational::new(i128::from(weight), u128::from(self.total_weight))?,
                ))
            })
            .collect()
    }

    pub fn marginal_counts(
        &self,
        output_id: &StableId,
    ) -> Result<BTreeMap<i64, u64>, DependenceError> {
        let mut counts = BTreeMap::new();
        for receipt in &self.receipts {
            let value = output_value(receipt, output_id)?;
            let entry = counts.entry(value).or_insert(0u64);
            *entry = entry
                .checked_add(receipt.state_weight)
                .ok_or(DependenceError::WeightOverflow)?;
        }
        Ok(counts)
    }
}

impl FactorGraphManifest {
    pub fn validate(&self) -> Result<u64, DependenceError> {
        if self.variables.is_empty() {
            return Err(DependenceError::EmptyVariables);
        }
        if self.distribution.states.is_empty() {
            return Err(DependenceError::EmptyStates);
        }
        if self.outputs.is_empty() {
            return Err(DependenceError::EmptyOutputs);
        }

        let mut variable_ids = BTreeSet::new();
        for variable in &self.variables {
            if !variable_ids.insert(variable.id.clone()) {
                return Err(DependenceError::DuplicateVariable);
            }
            if let FactorScope::DirectedDyad {
                initiator,
                recipient,
            } = &variable.scope
                && initiator == recipient
            {
                return Err(DependenceError::InvalidDirectedDyad);
            }
        }

        let mut output_ids = BTreeSet::new();
        for output in &self.outputs {
            if !output_ids.insert(output.id.clone()) {
                return Err(DependenceError::DuplicateOutput);
            }
            let mut terms = BTreeSet::new();
            for term in &output.terms {
                if !terms.insert(term.input.clone()) {
                    return Err(DependenceError::DuplicateTerm);
                }
                match &term.input {
                    StructuralInput::Exogenous(id) if !variable_ids.contains(id) => {
                        return Err(DependenceError::UnknownOutputInput);
                    }
                    StructuralInput::Intervention(_) => {}
                    StructuralInput::Exogenous(_) => {}
                }
            }
        }

        let mut total_weight = 0u64;
        let mut unique_joint_states = BTreeSet::new();
        for state in &self.distribution.states {
            if state.weight == 0 {
                return Err(DependenceError::ZeroStateWeight);
            }
            let mut state_ids = BTreeSet::new();
            for (id, _) in &state.values {
                if !variable_ids.contains(id) {
                    return Err(DependenceError::UnknownStateVariable);
                }
                if !state_ids.insert(id.clone()) {
                    return Err(DependenceError::DuplicateStateVariable);
                }
            }
            if state_ids != variable_ids {
                return Err(DependenceError::MissingStateVariable);
            }
            let mut canonical_state_values = state.values.clone();
            canonical_state_values.sort_by(|a, b| a.0.cmp(&b.0));
            if !unique_joint_states.insert(canonical_state_values) {
                return Err(DependenceError::DuplicateJointState);
            }
            total_weight = total_weight
                .checked_add(state.weight)
                .ok_or(DependenceError::WeightOverflow)?;
        }

        if total_weight == 0 {
            return Err(DependenceError::WeightTotalZero);
        }
        Ok(total_weight)
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, DependenceError> {
        self.validate()?;
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN);
        push_id(&mut out, &self.graph_id);
        push_id(&mut out, &self.version);
        push_id(&mut out, &self.distribution.evidence_ref);

        let mut variables = self.variables.iter().collect::<Vec<_>>();
        variables.sort_by(|a, b| a.id.cmp(&b.id));
        push_len(&mut out, variables.len());
        for variable in variables {
            push_id(&mut out, &variable.id);
            push_id(&mut out, &variable.source_id);
            out.extend_from_slice(&variable.source_profile_digest);
            push_id(&mut out, &variable.units);
            push_id(&mut out, &variable.equivalence_group);
            push_id(&mut out, &variable.invariance_justification);
            match &variable.scope {
                FactorScope::Scene => out.push(1),
                FactorScope::Group(group) => {
                    out.push(2);
                    push_id(&mut out, group);
                }
                FactorScope::Actor(actor) => {
                    out.push(3);
                    push_id(&mut out, actor);
                }
                FactorScope::DirectedDyad {
                    initiator,
                    recipient,
                } => {
                    out.push(4);
                    push_id(&mut out, initiator);
                    push_id(&mut out, recipient);
                }
                FactorScope::SpatialBasisCoefficient(basis) => {
                    out.push(5);
                    push_id(&mut out, basis);
                }
                FactorScope::TimeStep(tick) => {
                    out.push(6);
                    out.extend_from_slice(&tick.to_le_bytes());
                }
            }
        }

        let mut states = self.distribution.states.iter().collect::<Vec<_>>();
        states.sort_by(|a, b| {
            let mut left = a.values.clone();
            let mut right = b.values.clone();
            left.sort_by(|x, y| x.0.cmp(&y.0));
            right.sort_by(|x, y| x.0.cmp(&y.0));
            left.cmp(&right)
        });
        push_len(&mut out, states.len());
        for state in states {
            let mut values = state.values.iter().collect::<Vec<_>>();
            values.sort_by(|a, b| a.0.cmp(&b.0));
            push_len(&mut out, values.len());
            for (id, value) in values {
                push_id(&mut out, id);
                out.extend_from_slice(&value.to_le_bytes());
            }
            out.extend_from_slice(&state.weight.to_le_bytes());
        }

        let mut outputs = self.outputs.iter().collect::<Vec<_>>();
        outputs.sort_by(|a, b| a.id.cmp(&b.id));
        push_len(&mut out, outputs.len());
        for output in outputs {
            push_id(&mut out, &output.id);
            push_id(&mut out, &output.units);
            push_id(&mut out, &output.mechanism_id);
            push_id(&mut out, &output.evidence_ref);
            out.extend_from_slice(&output.intercept.to_le_bytes());
            let mut terms = output.terms.iter().collect::<Vec<_>>();
            terms.sort_by(|a, b| a.input.cmp(&b.input));
            push_len(&mut out, terms.len());
            for term in terms {
                match &term.input {
                    StructuralInput::Exogenous(id) => {
                        out.push(1);
                        push_id(&mut out, id);
                    }
                    StructuralInput::Intervention(id) => {
                        out.push(2);
                        push_id(&mut out, id);
                    }
                }
                out.extend_from_slice(&term.coefficient.to_le_bytes());
            }
        }
        Ok(out)
    }

    pub fn digest(&self) -> Result<[u8; 32], DependenceError> {
        Ok(*blake3::hash(&self.canonical_bytes()?).as_bytes())
    }

    pub fn evaluate(
        &self,
        intervention: &BranchIntervention,
    ) -> Result<BranchEvaluation, DependenceError> {
        let total_weight = self.validate()?;
        let mut intervention_values = BTreeMap::new();
        for (id, value) in &intervention.values {
            if intervention_values.insert(id.clone(), *value).is_some() {
                return Err(DependenceError::DuplicateIntervention);
            }
        }

        let mut required_interventions = BTreeSet::new();
        for output in &self.outputs {
            for term in &output.terms {
                if let StructuralInput::Intervention(id) = &term.input {
                    required_interventions.insert(id.clone());
                }
            }
        }
        for id in &required_interventions {
            if !intervention_values.contains_key(id) {
                return Err(DependenceError::MissingInterventionValue);
            }
        }
        for id in intervention_values.keys() {
            if !required_interventions.contains(id) {
                return Err(DependenceError::UnknownIntervention);
            }
        }

        let manifest_digest = self.digest()?;
        let mut receipts = Vec::with_capacity(self.distribution.states.len());
        let mut states = self.distribution.states.iter().collect::<Vec<_>>();
        states.sort_by(|a, b| {
            let mut left = a.values.clone();
            let mut right = b.values.clone();
            left.sort_by(|x, y| x.0.cmp(&y.0));
            right.sort_by(|x, y| x.0.cmp(&y.0));
            left.cmp(&right)
        });
        for (state_index, state) in states.into_iter().enumerate() {
            let latent_values = state.values.iter().cloned().collect::<BTreeMap<_, _>>();
            let mut output_values = Vec::with_capacity(self.outputs.len());

            for output in &self.outputs {
                let mut value = output.intercept;
                let mut terms = output.terms.iter().collect::<Vec<_>>();
                terms.sort_by(|a, b| a.input.cmp(&b.input));
                for term in terms {
                    let parent_value = match &term.input {
                        StructuralInput::Exogenous(id) => *latent_values
                            .get(id)
                            .ok_or(DependenceError::UnknownVariable)?,
                        StructuralInput::Intervention(id) => *intervention_values
                            .get(id)
                            .ok_or(DependenceError::MissingInterventionValue)?,
                    };
                    let contribution = parent_value
                        .checked_mul(term.coefficient)
                        .ok_or(DependenceError::ArithmeticOverflow)?;
                    value = value
                        .checked_add(contribution)
                        .ok_or(DependenceError::ArithmeticOverflow)?;
                }
                output_values.push((output.id.clone(), value));
            }
            output_values.sort_by(|a, b| a.0.cmp(&b.0));

            let mut receipt = DependenceReceipt {
                schema_id: "symtropy.road-dependence.receipt.v0.1",
                manifest_digest,
                branch_id: intervention.branch_id.clone(),
                state_index,
                state_weight: state.weight,
                latent_values: canonical_pairs(&state.values),
                intervention_values: canonical_pairs(&intervention.values),
                output_values,
                receipt_digest: [0; 32],
            };
            receipt.receipt_digest = *blake3::hash(&receipt_bytes(&receipt)).as_bytes();
            receipts.push(receipt);
        }

        Ok(BranchEvaluation {
            branch_id: intervention.branch_id.clone(),
            total_weight,
            receipts,
        })
    }
}

impl DependenceReceipt {
    /// Checks the unkeyed receipt checksum only. This detects accidental
    /// corruption; it is not authentication and does not re-evaluate the model.
    pub fn integrity_check(&self) -> bool {
        *blake3::hash(&receipt_bytes(self)).as_bytes() == self.receipt_digest
    }

    /// Recompute the complete receipt from the exact manifest and intervention.
    pub fn verify_against(
        &self,
        manifest: &FactorGraphManifest,
        intervention: &BranchIntervention,
    ) -> Result<(), DependenceError> {
        let evaluated = manifest.evaluate(intervention)?;
        if evaluated.receipts.iter().any(|candidate| candidate == self) {
            Ok(())
        } else {
            Err(DependenceError::ReceiptMismatch)
        }
    }
}

fn output_value(receipt: &DependenceReceipt, output_id: &StableId) -> Result<i64, DependenceError> {
    receipt
        .output_values
        .iter()
        .find(|(id, _)| id == output_id)
        .map(|(_, value)| *value)
        .ok_or(DependenceError::UnknownOutput)
}

fn canonical_pairs(pairs: &[(StableId, i64)]) -> Vec<(StableId, i64)> {
    let mut pairs = pairs.to_vec();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

fn receipt_bytes(receipt: &DependenceReceipt) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"symtropy.road-dependence.receipt.v0.1\0");
    push_bytes(&mut out, receipt.schema_id.as_bytes());
    out.extend_from_slice(&receipt.manifest_digest);
    push_id(&mut out, &receipt.branch_id);
    push_len(&mut out, receipt.state_index);
    out.extend_from_slice(&receipt.state_weight.to_le_bytes());
    for pairs in [
        &receipt.latent_values,
        &receipt.intervention_values,
        &receipt.output_values,
    ] {
        push_len(&mut out, pairs.len());
        for (id, value) in pairs {
            push_id(&mut out, id);
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

fn push_id(out: &mut Vec<u8>, id: &StableId) {
    let bytes = id.as_str().as_bytes();
    let len = u32::try_from(bytes.len()).expect("StableId length is bounded");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u64::try_from(bytes.len()).expect("fixture byte string must fit u64");
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(bytes);
}

fn push_len(out: &mut Vec<u8>, length: usize) {
    let len = u64::try_from(length).expect("fixture collection size must fit u64");
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

    fn id(s: &str) -> StableId {
        StableId::new(s).expect("fixture ID")
    }

    fn variable(id_value: &str, scope: FactorScope) -> FactorVariable {
        FactorVariable {
            id: id(id_value),
            source_id: StableId::new(format!("source-{id_value}")).expect("fixture ID"),
            source_profile_digest: *blake3::hash(id_value.as_bytes()).as_bytes(),
            units: id("dimensionless"),
            scope,
            equivalence_group: StableId::new(format!("equivalence-{id_value}"))
                .expect("fixture ID"),
            invariance_justification: StableId::new(format!("scm:shared-{id_value}"))
                .expect("fixture ID"),
        }
    }

    fn output(id_value: &str, left: &str, right: &str) -> LinearOutput {
        LinearOutput {
            id: id(id_value),
            units: id("dimensionless"),
            intercept: 0,
            terms: vec![
                LinearTerm {
                    input: StructuralInput::Exogenous(id(left)),
                    coefficient: 1,
                },
                LinearTerm {
                    input: StructuralInput::Exogenous(id(right)),
                    coefficient: 1,
                },
            ],
            mechanism_id: id("linear-sum-v1"),
            evidence_ref: id("fixture:analytic"),
        }
    }

    fn shared_factor_fixture() -> FactorGraphManifest {
        let variables = vec![
            variable("group-factor", FactorScope::Group(id("pedestrian-group-4"))),
            variable("local-a", FactorScope::Actor(id("ped-a"))),
            variable("local-b", FactorScope::Actor(id("ped-b"))),
        ];
        let mut states = Vec::new();
        for group in [-1, 1] {
            for local_a in [-1, 1] {
                for local_b in [-1, 1] {
                    states.push(WeightedJointState {
                        values: vec![
                            (id("group-factor"), group),
                            (id("local-a"), local_a),
                            (id("local-b"), local_b),
                        ],
                        weight: 1,
                    });
                }
            }
        }
        FactorGraphManifest {
            graph_id: id("pedestrian-shared-group-v1"),
            version: id("0.1.0"),
            variables,
            distribution: JointInnovationDistribution {
                states,
                evidence_ref: id("fixture:complete-enumeration"),
            },
            outputs: vec![
                output("response-a", "group-factor", "local-a"),
                output("response-b", "group-factor", "local-b"),
            ],
        }
    }

    fn independent_per_event_fixture() -> FactorGraphManifest {
        let mut variables = vec![
            variable("group-a", FactorScope::Actor(id("ped-a"))),
            variable("group-b", FactorScope::Actor(id("ped-b"))),
            variable("local-a", FactorScope::Actor(id("ped-a"))),
            variable("local-b", FactorScope::Actor(id("ped-b"))),
        ];
        variables.sort_by(|a, b| a.id.cmp(&b.id));
        let mut states = Vec::new();
        for group_a in [-1, 1] {
            for group_b in [-1, 1] {
                for local_a in [-1, 1] {
                    for local_b in [-1, 1] {
                        states.push(WeightedJointState {
                            values: vec![
                                (id("group-a"), group_a),
                                (id("group-b"), group_b),
                                (id("local-a"), local_a),
                                (id("local-b"), local_b),
                            ],
                            weight: 1,
                        });
                    }
                }
            }
        }
        FactorGraphManifest {
            graph_id: id("incorrect-independent-per-event-v1"),
            version: id("0.1.0"),
            variables,
            distribution: JointInnovationDistribution {
                states,
                evidence_ref: id("fixture:incorrect-independent-control"),
            },
            outputs: vec![
                output("response-a", "group-a", "local-a"),
                output("response-b", "group-b", "local-b"),
            ],
        }
    }

    #[test]
    fn shared_group_factor_produces_exact_covariance_one_and_marginal_variance_two() {
        let manifest = shared_factor_fixture();
        let evaluated = manifest
            .evaluate(&BranchIntervention {
                branch_id: id("continue"),
                values: vec![],
            })
            .unwrap();

        assert_eq!(evaluated.total_weight, 8);
        assert_eq!(
            evaluated
                .covariance(&id("response-a"), &id("response-b"))
                .unwrap(),
            ExactRational {
                numerator: 1,
                denominator: 1
            }
        );
        let variance_a = evaluated
            .covariance(&id("response-a"), &id("response-a"))
            .unwrap();
        assert_eq!(
            variance_a,
            ExactRational {
                numerator: 2,
                denominator: 1
            }
        );
        assert_eq!(
            evaluated.mean(&id("response-a")).unwrap(),
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
    }

    #[test]
    fn independent_per_event_control_has_same_marginals_but_wrong_joint_covariance() {
        let shared = shared_factor_fixture()
            .evaluate(&BranchIntervention {
                branch_id: id("continue"),
                values: vec![],
            })
            .unwrap();
        let independent = independent_per_event_fixture()
            .evaluate(&BranchIntervention {
                branch_id: id("continue"),
                values: vec![],
            })
            .unwrap();

        assert_eq!(
            shared.marginal_probabilities(&id("response-a")).unwrap(),
            independent
                .marginal_probabilities(&id("response-a"))
                .unwrap()
        );
        assert_eq!(
            shared.marginal_probabilities(&id("response-b")).unwrap(),
            independent
                .marginal_probabilities(&id("response-b"))
                .unwrap()
        );
        assert_eq!(
            shared
                .covariance(&id("response-a"), &id("response-b"))
                .unwrap()
                .numerator,
            1
        );
        assert_eq!(
            independent
                .covariance(&id("response-a"), &id("response-b"))
                .unwrap(),
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
    }

    #[test]
    fn intervention_changes_structural_descendant_but_not_shared_latent_factor() {
        let mut manifest = shared_factor_fixture();
        manifest.outputs.push(LinearOutput {
            id: id("pedestrian-gap-score"),
            units: id("dimensionless"),
            intercept: 0,
            terms: vec![
                LinearTerm {
                    input: StructuralInput::Exogenous(id("group-factor")),
                    coefficient: 1,
                },
                LinearTerm {
                    input: StructuralInput::Intervention(id("ego-yield-state")),
                    coefficient: 2,
                },
            ],
            mechanism_id: id("gap-score-v1"),
            evidence_ref: id("fixture:causal-descendant"),
        });
        let continue_run = manifest
            .evaluate(&BranchIntervention {
                branch_id: id("continue"),
                values: vec![(id("ego-yield-state"), 0)],
            })
            .unwrap();
        let yield_run = manifest
            .evaluate(&BranchIntervention {
                branch_id: id("yield"),
                values: vec![(id("ego-yield-state"), 1)],
            })
            .unwrap();

        for (continue_receipt, yield_receipt) in
            continue_run.receipts.iter().zip(&yield_run.receipts)
        {
            assert_eq!(continue_receipt.latent_values, yield_receipt.latent_values);
            assert_ne!(continue_receipt.output_values, yield_receipt.output_values);
            assert!(continue_receipt.integrity_check());
            assert!(yield_receipt.integrity_check());
            assert_eq!(
                continue_receipt.verify_against(
                    &manifest,
                    &BranchIntervention {
                        branch_id: id("continue"),
                        values: vec![(id("ego-yield-state"), 0)],
                    }
                ),
                Ok(())
            );
            assert_eq!(
                yield_receipt.verify_against(
                    &manifest,
                    &BranchIntervention {
                        branch_id: id("yield"),
                        values: vec![(id("ego-yield-state"), 1)],
                    }
                ),
                Ok(())
            );
        }
    }

    #[test]
    fn canonical_digest_is_independent_of_declaration_order() {
        let first = shared_factor_fixture();
        let mut second = first.clone();
        second.variables.reverse();
        second.outputs.reverse();
        for state in &mut second.distribution.states {
            state.values.reverse();
        }
        second.distribution.states.reverse();
        assert_eq!(first.digest().unwrap(), second.digest().unwrap());
    }

    #[test]
    fn canonical_evaluation_receipts_ignore_state_declaration_order() {
        let first_manifest = shared_factor_fixture();
        let first = first_manifest
            .evaluate(&BranchIntervention {
                branch_id: id("continue"),
                values: vec![],
            })
            .unwrap();
        let mut second_manifest = first_manifest.clone();
        second_manifest.variables.reverse();
        second_manifest.distribution.states.reverse();
        for state in &mut second_manifest.distribution.states {
            state.values.reverse();
        }
        let second = second_manifest
            .evaluate(&BranchIntervention {
                branch_id: id("continue"),
                values: vec![],
            })
            .unwrap();
        assert_eq!(first.receipts, second.receipts);
    }

    #[test]
    fn rational_zero_normalizes_without_large_denominator_conversion() {
        assert_eq!(
            ExactRational::new(0, u128::MAX).unwrap(),
            ExactRational {
                numerator: 0,
                denominator: 1
            }
        );
    }

    #[test]
    fn missing_factor_value_is_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest.distribution.states[0].values.pop();
        assert_eq!(
            manifest.validate(),
            Err(DependenceError::MissingStateVariable)
        );
    }

    #[test]
    fn changing_source_profile_digest_changes_manifest_identity() {
        let first = shared_factor_fixture();
        let mut changed = first.clone();
        changed.variables[0].source_profile_digest[0] ^= 1;
        assert_ne!(first.digest().unwrap(), changed.digest().unwrap());
    }

    #[test]
    fn duplicate_factor_variable_is_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest.variables.push(manifest.variables[0].clone());
        assert_eq!(manifest.validate(), Err(DependenceError::DuplicateVariable));
    }

    #[test]
    fn zero_probability_state_is_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest.distribution.states[0].weight = 0;
        assert_eq!(manifest.validate(), Err(DependenceError::ZeroStateWeight));
    }

    #[test]
    fn incomplete_intervention_is_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest.outputs.push(LinearOutput {
            id: id("descendant"),
            units: id("dimensionless"),
            intercept: 0,
            terms: vec![LinearTerm {
                input: StructuralInput::Intervention(id("decision")),
                coefficient: 1,
            }],
            mechanism_id: id("decision-v1"),
            evidence_ref: id("fixture:decision"),
        });
        assert_eq!(
            manifest.evaluate(&BranchIntervention {
                branch_id: id("branch-a"),
                values: vec![],
            }),
            Err(DependenceError::MissingInterventionValue)
        );
    }

    #[test]
    fn receipt_tampering_is_detected() {
        let manifest = shared_factor_fixture();
        let mut evaluated = manifest
            .evaluate(&BranchIntervention {
                branch_id: id("branch-a"),
                values: vec![],
            })
            .unwrap();
        evaluated.receipts[0].output_values[0].1 ^= 1;
        assert!(!evaluated.receipts[0].integrity_check());
        assert_eq!(
            evaluated.receipts[0].verify_against(
                &manifest,
                &BranchIntervention {
                    branch_id: id("branch-a"),
                    values: vec![],
                }
            ),
            Err(DependenceError::ReceiptMismatch)
        );
    }

    #[test]
    fn duplicate_joint_state_rows_are_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest
            .distribution
            .states
            .push(manifest.distribution.states[0].clone());
        assert_eq!(
            manifest.validate(),
            Err(DependenceError::DuplicateJointState)
        );
    }

    #[test]
    fn intervention_value_change_invalidates_receipt_replay() {
        let mut manifest = shared_factor_fixture();
        manifest.outputs.push(LinearOutput {
            id: id("decision-descendant"),
            units: id("dimensionless"),
            intercept: 0,
            terms: vec![LinearTerm {
                input: StructuralInput::Intervention(id("action")),
                coefficient: 1,
            }],
            mechanism_id: id("action-dependent-v1"),
            evidence_ref: id("fixture:replay-binding"),
        });
        let original = BranchIntervention {
            branch_id: id("branch-a"),
            values: vec![(id("action"), 0)],
        };
        let changed = BranchIntervention {
            branch_id: id("branch-a"),
            values: vec![(id("action"), 1)],
        };
        let receipt = manifest.evaluate(&original).unwrap().receipts[0].clone();
        assert_eq!(
            receipt.verify_against(&manifest, &changed),
            Err(DependenceError::ReceiptMismatch)
        );
    }

    #[test]
    fn joint_weight_accumulation_overflow_is_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest.distribution.states[0].weight = u64::MAX;
        manifest.distribution.states[1].weight = 1;
        assert_eq!(manifest.validate(), Err(DependenceError::WeightOverflow));
    }

    #[test]
    fn structural_output_arithmetic_overflow_is_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest.outputs[0].intercept = i64::MAX;
        assert_eq!(
            manifest.evaluate(&BranchIntervention {
                branch_id: id("continue"),
                values: vec![],
            }),
            Err(DependenceError::ArithmeticOverflow)
        );
    }

    #[test]
    fn unknown_exogenous_parent_is_rejected_before_evaluation() {
        let mut manifest = shared_factor_fixture();
        manifest.outputs[0].terms[0].input =
            StructuralInput::Exogenous(id("missing-latent-factor"));
        assert_eq!(
            manifest.validate(),
            Err(DependenceError::UnknownOutputInput)
        );
    }

    #[test]
    fn invalid_self_directed_dyad_is_rejected() {
        let mut manifest = shared_factor_fixture();
        manifest.variables[0].scope = FactorScope::DirectedDyad {
            initiator: id("ped-a"),
            recipient: id("ped-a"),
        };
        assert_eq!(
            manifest.validate(),
            Err(DependenceError::InvalidDirectedDyad)
        );
    }
}
