//! Exact all-horizon observational equivalence for finite stochastic mechanisms.
//!
//! Each hypothesis induces a weighted probabilistic transducer over joint
//! physical/sensor states. Pairwise equivalence is decided by closing the span
//! of reachable signed probability-difference vectors under every action and
//! observable outcome operator. All rank decisions use exact rational
//! arithmetic; there are no floating-point tolerances.

use super::{JointKernel, JointMass, Rational};
use std::collections::{BTreeSet, VecDeque};

const MAX_AMBIENT_DIMENSION: usize = 8_192;
const MAX_BASIS_VECTORS: usize = 8_192;
const MAX_PAIRWISE_HYPOTHESES: usize = 32;

/// Exact signed rational value used by certificates and distinguishing witnesses.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct SignedRational {
    negative: bool,
    magnitude: Rational,
}

impl SignedRational {
    pub const fn zero() -> Self {
        Self {
            negative: false,
            magnitude: Rational::zero(),
        }
    }

    pub fn positive(magnitude: Rational) -> Self {
        Self {
            negative: false,
            magnitude,
        }
    }

    pub fn negative(magnitude: Rational) -> Self {
        Self {
            negative: !magnitude.is_zero(),
            magnitude,
        }
    }

    pub const fn is_zero(self) -> bool {
        self.magnitude.is_zero()
    }

    pub const fn is_negative(self) -> bool {
        self.negative
    }

    pub const fn numerator(self) -> u128 {
        self.magnitude.numerator()
    }

    pub const fn denominator(self) -> u128 {
        self.magnitude.denominator()
    }

    pub fn checked_add(self, other: Self) -> Result<Self, String> {
        if self.is_zero() {
            return Ok(other);
        }
        if other.is_zero() {
            return Ok(self);
        }
        if self.negative == other.negative {
            return Ok(Self {
                negative: self.negative,
                magnitude: self.magnitude.checked_add(other.magnitude)?,
            });
        }

        match self.magnitude.cmp(&other.magnitude) {
            std::cmp::Ordering::Greater => Ok(Self {
                negative: self.negative,
                magnitude: self.magnitude.checked_sub(other.magnitude)?,
            }),
            std::cmp::Ordering::Less => Ok(Self {
                negative: other.negative,
                magnitude: other.magnitude.checked_sub(self.magnitude)?,
            }),
            std::cmp::Ordering::Equal => Ok(Self::zero()),
        }
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, String> {
        self.checked_add(Self {
            negative: !other.negative && !other.is_zero(),
            magnitude: other.magnitude,
        })
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, String> {
        Ok(Self {
            negative: (self.negative != other.negative)
                && !self.is_zero()
                && !other.is_zero(),
            magnitude: self.magnitude.checked_mul(other.magnitude)?,
        })
    }

    pub fn checked_mul_probability(self, probability: Rational) -> Result<Self, String> {
        Ok(Self {
            negative: self.negative && !self.is_zero() && !probability.is_zero(),
            magnitude: self.magnitude.checked_mul(probability)?,
        })
    }

    pub fn checked_div(self, other: Self) -> Result<Self, String> {
        if other.is_zero() {
            return Err("signed rational division by zero".into());
        }
        Ok(Self {
            negative: (self.negative != other.negative) && !self.is_zero(),
            magnitude: self.magnitude.checked_div(other.magnitude)?,
        })
    }

    fn normalized(self) -> Self {
        if self.is_zero() {
            Self::zero()
        } else {
            self
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentifiabilityConfig {
    pub max_ambient_dimension: usize,
    pub max_basis_vectors: usize,
    pub max_transition_applications: usize,
}

impl Default for IdentifiabilityConfig {
    fn default() -> Self {
        Self {
            max_ambient_dimension: 2_048,
            max_basis_vectors: 2_048,
            max_transition_applications: 500_000,
        }
    }
}

/// One public output symbol emitted by an experiment.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum OutcomeSymbol {
    Observation { index: usize, id: String },
    Hazard,
}

/// One step in a concrete distinguishing action/output word.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistinguishingStep {
    pub action_id: String,
    pub outcome: OutcomeSymbol,
}

/// A concrete finite action/output word with a non-zero exact probability difference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistinguishingWitness {
    pub left_hypothesis_id: String,
    pub right_hypothesis_id: String,
    pub steps: Vec<DistinguishingStep>,
    /// P(word | left) - P(word | right).
    pub probability_difference: SignedRational,
    pub left_probability: Rational,
    pub right_probability: Rational,
}

/// A closure certificate proving all-horizon equivalence for one hypothesis pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EquivalenceCertificate {
    pub left_hypothesis_id: String,
    pub right_hypothesis_id: String,
    /// Includes one additional absorbing state per hypothesis for terminal hazard.
    pub ambient_dimension: usize,
    /// Exact row-space basis. Pivot columns form an identity matrix.
    pub basis_vectors: Vec<Vec<SignedRational>>,
    pub pivot_columns: Vec<usize>,
    pub operators_checked: usize,
}

/// Exact classification for one pair of hypotheses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PairClassification {
    Distinguishable(DistinguishingWitness),
    Equivalent(EquivalenceCertificate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairIdentifiability {
    pub left_hypothesis_id: String,
    pub right_hypothesis_id: String,
    pub classification: PairClassification,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentifiabilityReport {
    /// Hypotheses with positive mass in the supplied initial joint belief.
    pub supported_hypotheses: Vec<String>,
    /// Hypotheses present in the semantic vocabulary but absent from prior support.
    pub zero_prior_hypotheses: Vec<String>,
    pub pairwise: Vec<PairIdentifiability>,
    /// Pairwise all-horizon equivalence classes over supported hypotheses.
    pub equivalence_classes: Vec<Vec<String>>,
}

#[derive(Clone, Debug)]
struct BasisVector {
    pivot: usize,
    values: Vec<SignedRational>,
}

#[derive(Clone, Debug, Default)]
struct LinearBasis {
    vectors: Vec<BasisVector>,
}

impl LinearBasis {
    fn insert(
        &mut self,
        mut candidate: Vec<SignedRational>,
        max_basis_vectors: usize,
    ) -> Result<Option<usize>, String> {
        for basis in &self.vectors {
            let factor = candidate[basis.pivot];
            if factor.is_zero() {
                continue;
            }
            for (value, basis_value) in candidate.iter_mut().zip(basis.values.iter()) {
                *value = value.checked_sub(factor.checked_mul(*basis_value)?)?;
            }
        }

        let Some(pivot) = candidate.iter().position(|value| !value.is_zero()) else {
            return Ok(None);
        };

        let divisor = candidate[pivot];
        for value in &mut candidate {
            *value = value.checked_div(divisor)?.normalized();
        }

        // Maintain identity values in all pivot columns. This lets the verifier
        // test membership by exact elimination without rerunning basis discovery.
        for basis in &mut self.vectors {
            let factor = basis.values[pivot];
            if factor.is_zero() {
                continue;
            }
            for (value, new_value) in basis.values.iter_mut().zip(candidate.iter()) {
                *value = value.checked_sub(factor.checked_mul(*new_value)?)?;
            }
        }

        if self.vectors.len() >= max_basis_vectors {
            return Err("identifiability basis-vector budget exceeded".into());
        }
        self.vectors.push(BasisVector {
            pivot,
            values: candidate,
        });
        self.vectors.sort_by_key(|basis| basis.pivot);
        Ok(Some(pivot))
    }

    fn contains(&self, mut candidate: Vec<SignedRational>) -> Result<bool, String> {
        for basis in &self.vectors {
            let factor = candidate[basis.pivot];
            if factor.is_zero() {
                continue;
            }
            for (value, basis_value) in candidate.iter_mut().zip(basis.values.iter()) {
                *value = value.checked_sub(factor.checked_mul(*basis_value)?)?;
            }
        }
        Ok(candidate.iter().all(|value| value.is_zero()))
    }
}

#[derive(Clone, Debug)]
struct QueueEntry {
    vector: Vec<SignedRational>,
    witness: Vec<DistinguishingStep>,
}

fn local_state_count(kernel: &JointKernel) -> Result<usize, String> {
    kernel.physical_state_count
        .checked_mul(kernel.sensor_state_count)
        .ok_or_else(|| "identifiability joint-state count overflow".to_string())
}

fn ambient_dimension(kernel: &JointKernel) -> Result<usize, String> {
    local_state_count(kernel)?
        .checked_add(1)
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| "identifiability ambient-dimension overflow".to_string())
}

fn hypothesis_conditional_belief(
    kernel: &JointKernel,
    hypothesis: usize,
) -> Result<(Rational, Vec<Rational>), String> {
    let states = local_state_count(kernel)?;
    let mut masses = vec![Rational::zero(); states];
    let mut total = Rational::zero();
    for point in &kernel.initial_belief {
        if point.hypothesis != hypothesis {
            continue;
        }
        let state = point.physical_state * kernel.sensor_state_count + point.sensor_state;
        masses[state] = masses[state].checked_add(point.mass)?;
        total = total.checked_add(point.mass)?;
    }
    if total.is_zero() {
        return Ok((Rational::zero(), masses));
    }
    for mass in &mut masses {
        *mass = mass.checked_div(total)?;
    }
    Ok((total, masses))
}

fn initial_difference(
    kernel: &JointKernel,
    left: usize,
    right: usize,
) -> Result<(Vec<SignedRational>, Rational, Rational), String> {
    let (left_prior_mass, left_belief) = hypothesis_conditional_belief(kernel, left)?;
    let (right_prior_mass, right_belief) = hypothesis_conditional_belief(kernel, right)?;
    if left_prior_mass.is_zero() || right_prior_mass.is_zero() {
        return Err("identifiability pair must have positive prior support for both hypotheses".into());
    }

    let states = local_state_count(kernel)?;
    let side_dimension = states + 1;
    let mut difference = vec![SignedRational::zero(); side_dimension * 2];
    for state in 0..states {
        difference[state] = SignedRational::positive(left_belief[state]);
        difference[side_dimension + state] = SignedRational::negative(right_belief[state]);
    }
    Ok((difference, left_prior_mass, right_prior_mass))
}

fn outcome_symbols(kernel: &JointKernel) -> Vec<OutcomeSymbol> {
    let mut symbols: Vec<OutcomeSymbol> = kernel
        .observation_ids
        .iter()
        .enumerate()
        .map(|(index, id)| OutcomeSymbol::Observation {
            index,
            id: id.clone(),
        })
        .collect();
    symbols.push(OutcomeSymbol::Hazard);
    symbols
}

fn apply_operator(
    kernel: &JointKernel,
    left_hypothesis: usize,
    right_hypothesis: usize,
    vector: &[SignedRational],
    action: usize,
    outcome_symbol: &OutcomeSymbol,
) -> Result<Vec<SignedRational>, String> {
    let states = local_state_count(kernel)?;
    let side_dimension = states + 1;
    let dimension = side_dimension * 2;
    if vector.len() != dimension {
        return Err("identifiability vector has an invalid dimension".into());
    }

    let mut next = vec![SignedRational::zero(); dimension];
    for side in 0..2 {
        let hypothesis = if side == 0 {
            left_hypothesis
        } else {
            right_hypothesis
        };
        for state in 0..states {
            let coefficient = vector[side * side_dimension + state];
            if coefficient.is_zero() {
                continue;
            }
            let physical_state = state / kernel.sensor_state_count;
            let sensor_state = state % kernel.sensor_state_count;
            let row_index = kernel.row_index(
                hypothesis,
                physical_state,
                sensor_state,
                action,
            );
            let row = &kernel.rows[row_index];

            for outcome in &row.outcomes {
                let matches = match (outcome_symbol, outcome.observation) {
                    (OutcomeSymbol::Hazard, None) => true,
                    (OutcomeSymbol::Observation { index, .. }, Some(observation)) => {
                        *index == observation
                    }
                    _ => false,
                };
                if !matches {
                    continue;
                }

                let destination_state = match outcome.observation {
                    Some(_) => {
                        outcome.next_physical_state * kernel.sensor_state_count
                            + outcome.next_sensor_state
                    }
                    None => states, // terminal hazard absorbing state
                };
                let destination = side * side_dimension + destination_state;
                let contribution =
                    coefficient.checked_mul_probability(outcome.probability)?;
                next[destination] = next[destination].checked_add(contribution)?;
            }
        }
    }
    Ok(next)
}

fn output_difference(vector: &[SignedRational]) -> Result<SignedRational, String> {
    vector.iter().try_fold(
        SignedRational::zero(),
        |sum, value| sum.checked_add(*value),
    )
}

fn step_for(
    kernel: &JointKernel,
    action: usize,
    outcome: &OutcomeSymbol,
) -> DistinguishingStep {
    DistinguishingStep {
        action_id: kernel.action_ids[action].clone(),
        outcome: outcome.clone(),
    }
}

fn replay_word_probability(
    kernel: &JointKernel,
    hypothesis: usize,
    steps: &[DistinguishingStep],
) -> Result<Rational, String> {
    let (prior_mass, mut belief) = hypothesis_conditional_belief(kernel, hypothesis)?;
    if prior_mass.is_zero() {
        return Err("cannot replay a witness for a zero-prior hypothesis".into());
    }
    let states = local_state_count(kernel)?;
    let side_dimension = states + 1;

    for step in steps {
        let action = kernel
            .action_ids
            .iter()
            .position(|id| id == &step.action_id)
            .ok_or_else(|| "distinguishing witness contains an unknown action".to_string())?;
        let mut next = vec![Rational::zero(); side_dimension];
        for state in 0..states {
            if belief[state].is_zero() {
                continue;
            }
            let physical_state = state / kernel.sensor_state_count;
            let sensor_state = state % kernel.sensor_state_count;
            let row_index = kernel.row_index(hypothesis, physical_state, sensor_state, action);
            for outcome in &kernel.rows[row_index].outcomes {
                let matches = match (&step.outcome, outcome.observation) {
                    (OutcomeSymbol::Hazard, None) => true,
                    (OutcomeSymbol::Observation { index, .. }, Some(observed)) => {
                        *index == observed
                    }
                    _ => false,
                };
                if !matches {
                    continue;
                }
                let destination = match outcome.observation {
                    Some(_) => {
                        outcome.next_physical_state * kernel.sensor_state_count
                            + outcome.next_sensor_state
                    }
                    None => states,
                };
                let contribution = belief[state].checked_mul(outcome.probability)?;
                next[destination] = next[destination].checked_add(contribution)?;
            }
        }
        belief = next;
    }
    belief.into_iter().try_fold(
        Rational::zero(),
        |sum, mass| sum.checked_add(mass),
    )
}

/// Independently replay a distinguishing witness under both hypotheses.
pub fn verify_distinguishing_witness(
    kernel: &JointKernel,
    witness: &DistinguishingWitness,
) -> Result<bool, String> {
    kernel.validate()?;
    let left = kernel
        .hypothesis_ids
        .iter()
        .position(|id| id == &witness.left_hypothesis_id)
        .ok_or_else(|| "witness references an unknown left hypothesis".to_string())?;
    let right = kernel
        .hypothesis_ids
        .iter()
        .position(|id| id == &witness.right_hypothesis_id)
        .ok_or_else(|| "witness references an unknown right hypothesis".to_string())?;
    let left_probability = replay_word_probability(kernel, left, &witness.steps)?;
    let right_probability = replay_word_probability(kernel, right, &witness.steps)?;
    let difference = if left_probability >= right_probability {
        SignedRational::positive(left_probability.checked_sub(right_probability)?)
    } else {
        SignedRational::negative(right_probability.checked_sub(left_probability)?)
    };
    Ok(!difference.is_zero()
        && difference == witness.probability_difference
        && left_probability == witness.left_probability
        && right_probability == witness.right_probability)
}

/// Verify an all-horizon equivalence proof by checking span inclusion, zero output,
/// and closure under every action/outcome operator. This does not rerun basis search.
pub fn verify_equivalence_certificate(
    kernel: &JointKernel,
    certificate: &EquivalenceCertificate,
) -> Result<bool, String> {
    kernel.validate()?;
    let left = kernel
        .hypothesis_ids
        .iter()
        .position(|id| id == &certificate.left_hypothesis_id)
        .ok_or_else(|| "certificate references an unknown left hypothesis".to_string())?;
    let right = kernel
        .hypothesis_ids
        .iter()
        .position(|id| id == &certificate.right_hypothesis_id)
        .ok_or_else(|| "certificate references an unknown right hypothesis".to_string())?;
    let expected_dimension = ambient_dimension(kernel)?;
    if certificate.ambient_dimension != expected_dimension
        || certificate.basis_vectors.len() != certificate.pivot_columns.len()
        || certificate.basis_vectors.len() > MAX_BASIS_VECTORS
    {
        return Ok(false);
    }

    let mut seen_pivots = BTreeSet::new();
    for (row_index, vector) in certificate.basis_vectors.iter().enumerate() {
        if vector.len() != expected_dimension {
            return Ok(false);
        }
        let pivot = certificate.pivot_columns[row_index];
        if pivot >= expected_dimension || !seen_pivots.insert(pivot) {
            return Ok(false);
        }
        if vector[pivot] != SignedRational::positive(Rational::one()) {
            return Ok(false);
        }
        for (other_index, other_pivot) in certificate.pivot_columns.iter().enumerate() {
            if other_index != row_index && !vector[*other_pivot].is_zero() {
                return Ok(false);
            }
        }
        if !output_difference(vector)?.is_zero() {
            return Ok(false);
        }
    }

    let (initial, _, _) = initial_difference(kernel, left, right)?;
    let basis = certificate_basis(certificate)?;
    if !basis.contains(initial)? {
        return Ok(false);
    }

    let symbols = outcome_symbols(kernel);
    let expected_operators = kernel
        .action_ids
        .len()
        .checked_mul(symbols.len())
        .and_then(|n| n.checked_mul(certificate.basis_vectors.len()))
        .ok_or_else(|| "certificate operator count overflow".to_string())?;
    if expected_operators != certificate.operators_checked {
        return Ok(false);
    }

    for vector in &certificate.basis_vectors {
        for action in 0..kernel.action_ids.len() {
            for symbol in &symbols {
                let next = apply_operator(
                    kernel,
                    left,
                    right,
                    vector,
                    action,
                    symbol,
                )?;
                if !basis.contains(next)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(true)
}

fn certificate_basis(
    certificate: &EquivalenceCertificate,
) -> Result<LinearBasis, String> {
    let mut basis = LinearBasis::default();
    for (pivot, vector) in certificate
        .pivot_columns
        .iter()
        .copied()
        .zip(certificate.basis_vectors.iter().cloned())
    {
        if vector.get(pivot).is_none() {
            return Err("certificate pivot is out of range".into());
        }
        basis.vectors.push(BasisVector {
            pivot,
            values: vector,
        });
    }
    basis.vectors.sort_by_key(|row| row.pivot);
    Ok(basis)
}

/// Analyze one positive-prior hypothesis pair. The result is exact and all-horizon.
pub fn analyze_hypothesis_pair(
    kernel: &JointKernel,
    left_hypothesis: usize,
    right_hypothesis: usize,
    config: &IdentifiabilityConfig,
) -> Result<PairClassification, String> {
    kernel.validate()?;
    if left_hypothesis >= kernel.hypothesis_ids.len()
        || right_hypothesis >= kernel.hypothesis_ids.len()
        || left_hypothesis == right_hypothesis
    {
        return Err("identifiability pair indices are invalid or identical".into());
    }
    if config.max_ambient_dimension == 0
        || config.max_basis_vectors == 0
        || config.max_transition_applications == 0
    {
        return Err("identifiability resource caps must be positive".into());
    }

    let dimension = ambient_dimension(kernel)?;
    if dimension > config.max_ambient_dimension || dimension > MAX_AMBIENT_DIMENSION {
        return Err("identifiability ambient-dimension budget exceeded".into());
    }
    let (initial, _, _) =
        initial_difference(kernel, left_hypothesis, right_hypothesis)?;

    let mut basis = LinearBasis::default();
    let mut queue = VecDeque::new();
    if basis
        .insert(initial.clone(), config.max_basis_vectors)?
        .is_some()
    {
        queue.push_back(QueueEntry {
            vector: initial,
            witness: Vec::new(),
        });
    }

    let symbols = outcome_symbols(kernel);
    let mut operators_checked = 0_usize;
    while let Some(entry) = queue.pop_front() {
        for action in 0..kernel.action_ids.len() {
            for symbol in &symbols {
                operators_checked = operators_checked
                    .checked_add(1)
                    .ok_or_else(|| "identifiability operator counter overflow".to_string())?;
                if operators_checked > config.max_transition_applications {
                    return Err("identifiability transition-application budget exceeded".into());
                }

                let next = apply_operator(
                    kernel,
                    left_hypothesis,
                    right_hypothesis,
                    &entry.vector,
                    action,
                    symbol,
                )?;
                let difference = output_difference(&next)?;
                let mut witness_steps = entry.witness.clone();
                witness_steps.push(step_for(kernel, action, symbol));

                if !difference.is_zero() {
                    let left_probability = replay_word_probability(
                        kernel,
                        left_hypothesis,
                        &witness_steps,
                    )?;
                    let right_probability = replay_word_probability(
                        kernel,
                        right_hypothesis,
                        &witness_steps,
                    )?;
                    let exact_difference = if left_probability >= right_probability {
                        SignedRational::positive(
                            left_probability.checked_sub(right_probability)?,
                        )
                    } else {
                        SignedRational::negative(
                            right_probability.checked_sub(left_probability)?,
                        )
                    };
                    if exact_difference != difference {
                        return Err("internal distinguishing witness replay mismatch".into());
                    }
                    return Ok(PairClassification::Distinguishable(
                        DistinguishingWitness {
                            left_hypothesis_id: kernel.hypothesis_ids[left_hypothesis].clone(),
                            right_hypothesis_id: kernel.hypothesis_ids[right_hypothesis].clone(),
                            steps: witness_steps,
                            probability_difference: difference,
                            left_probability,
                            right_probability,
                        },
                    ));
                }

                if basis
                    .insert(next.clone(), config.max_basis_vectors)?
                    .is_some()
                {
                    queue.push_back(QueueEntry {
                        vector: next,
                        witness: witness_steps,
                    });
                }
            }
        }
    }

    let certificate = EquivalenceCertificate {
        left_hypothesis_id: kernel.hypothesis_ids[left_hypothesis].clone(),
        right_hypothesis_id: kernel.hypothesis_ids[right_hypothesis].clone(),
        ambient_dimension: dimension,
        basis_vectors: basis
            .vectors
            .iter()
            .map(|row| row.values.clone())
            .collect(),
        pivot_columns: basis.vectors.iter().map(|row| row.pivot).collect(),
        operators_checked,
    };
    if !verify_equivalence_certificate(kernel, &certificate)? {
        return Err("generated equivalence certificate failed independent closure verification".into());
    }
    Ok(PairClassification::Equivalent(certificate))
}

/// Analyze all positive-prior hypotheses and form exact equivalence classes.
pub fn analyze_identifiability(
    kernel: &JointKernel,
    config: &IdentifiabilityConfig,
) -> Result<IdentifiabilityReport, String> {
    kernel.validate()?;
    let mut supported = Vec::new();
    let mut zero_prior = Vec::new();
    let mut support_indices = Vec::new();
    for hypothesis in 0..kernel.hypothesis_ids.len() {
        let (mass, _) = hypothesis_conditional_belief(kernel, hypothesis)?;
        if mass.is_zero() {
            zero_prior.push(kernel.hypothesis_ids[hypothesis].clone());
        } else {
            supported.push(kernel.hypothesis_ids[hypothesis].clone());
            support_indices.push(hypothesis);
        }
    }
    if support_indices.len() > MAX_PAIRWISE_HYPOTHESES {
        return Err("pairwise identifiability hypothesis budget exceeded".into());
    }

    let mut pairwise = Vec::new();
    let mut union_find = UnionFind::new(support_indices.len());
    for left_position in 0..support_indices.len() {
        for right_position in (left_position + 1)..support_indices.len() {
            let left = support_indices[left_position];
            let right = support_indices[right_position];
            let classification =
                analyze_hypothesis_pair(kernel, left, right, config)?;
            if matches!(classification, PairClassification::Equivalent(_)) {
                union_find.union(left_position, right_position);
            }
            pairwise.push(PairIdentifiability {
                left_hypothesis_id: kernel.hypothesis_ids[left].clone(),
                right_hypothesis_id: kernel.hypothesis_ids[right].clone(),
                classification,
            });
        }
    }

    let mut classes_by_root: Vec<(usize, Vec<String>)> = Vec::new();
    for (position, hypothesis) in supported.iter().enumerate() {
        let root = union_find.find(position);
        if let Some((_, members)) = classes_by_root.iter_mut().find(|(id, _)| *id == root) {
            members.push(hypothesis.clone());
        } else {
            classes_by_root.push((root, vec![hypothesis.clone()]));
        }
    }
    classes_by_root.sort_by_key(|(_, members)| {
        supported.iter().position(|id| id == &members[0]).unwrap_or(usize::MAX)
    });
    let equivalence_classes = classes_by_root
        .into_iter()
        .map(|(_, members)| members)
        .collect();

    Ok(IdentifiabilityReport {
        supported_hypotheses: supported,
        zero_prior_hypotheses: zero_prior,
        pairwise,
        equivalence_classes,
    })
}

#[derive(Clone, Debug)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, item: usize) -> usize {
        if self.parent[item] != item {
            self.parent[item] = self.find(self.parent[item]);
        }
        self.parent[item]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        if self.rank[left_root] < self.rank[right_root] {
            self.parent[left_root] = right_root;
        } else if self.rank[left_root] > self.rank[right_root] {
            self.parent[right_root] = left_root;
        } else {
            self.parent[right_root] = left_root;
            self.rank[left_root] += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointOutcome, KernelRow};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn quarter() -> Rational {
        Rational::new(1, 4).unwrap()
    }

    fn three_quarters() -> Rational {
        Rational::new(3, 4).unwrap()
    }

    fn one_third() -> Rational {
        Rational::new(1, 3).unwrap()
    }

    fn two_thirds() -> Rational {
        Rational::new(2, 3).unwrap()
    }

    fn make_kernel(
        hypotheses: usize,
        physical_states: usize,
        sensor_states: usize,
        actions: Vec<&str>,
        observations: Vec<&str>,
        initial_belief: Vec<JointMass>,
        mut outcomes: impl FnMut(usize, usize, usize, usize) -> Vec<JointOutcome>,
    ) -> JointKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..hypotheses {
            for physical_state in 0..physical_states {
                for sensor_state in 0..sensor_states {
                    for action in 0..actions.len() {
                        rows.push(KernelRow {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            action,
                            outcomes: outcomes(hypothesis, physical_state, sensor_state, action),
                        });
                    }
                }
            }
        }
        JointKernel {
            hypothesis_ids: (0..hypotheses)
                .map(|index| format!("h{index}"))
                .collect(),
            physical_state_count: physical_states,
            sensor_state_count: sensor_states,
            action_ids: actions.into_iter().map(str::to_string).collect(),
            observation_ids: observations.into_iter().map(str::to_string).collect(),
            action_costs: vec![1; actions.len()],
            initial_belief,
            rows,
        }
    }

    fn config() -> IdentifiabilityConfig {
        IdentifiabilityConfig {
            max_ambient_dimension: 64,
            max_basis_vectors: 64,
            max_transition_applications: 100_000,
        }
    }

    fn equivalent_different_hidden_states() -> JointKernel {
        make_kernel(
            2,
            2,
            2,
            vec!["read", "condition"],
            vec!["same"],
            vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 1,
                    sensor_state: 1,
                    mass: half(),
                },
            ],
            |_hypothesis, physical, sensor, action| {
                let next_physical = if action == 0 { physical } else { 1 - physical };
                let next_sensor = if action == 0 { sensor } else { 1 - sensor };
                vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: next_physical,
                    next_sensor_state: next_sensor,
                    observation: Some(0),
                }]
            },
        )
    }

    fn multi_step_identifying_kernel() -> JointKernel {
        make_kernel(
            2,
            2,
            1,
            vec!["prime", "read"],
            vec!["zero", "one"],
            vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
            ],
            |hypothesis, physical, sensor, action| {
                let observation = if action == 0 {
                    0
                } else if physical == 0 || hypothesis == 0 {
                    0
                } else {
                    1
                };
                let next_physical = if action == 0 { 1 } else { physical };
                vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: next_physical,
                    next_sensor_state: sensor,
                    observation: Some(observation),
                }]
            },
        )
    }

    fn hazard_identifying_kernel() -> JointKernel {
        make_kernel(
            2,
            1,
            1,
            vec!["stress-test"],
            vec!["normal"],
            vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
            ],
            |hypothesis, physical, sensor, _action| {
                if hypothesis == 0 {
                    vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: physical,
                        next_sensor_state: sensor,
                        observation: Some(0),
                    }]
                } else {
                    vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: physical,
                        next_sensor_state: sensor,
                        observation: None,
                    }]
                }
            },
        )
    }

    fn correlation_only_kernel() -> JointKernel {
        make_kernel(
            2,
            2,
            1,
            vec!["tap", "read"],
            vec!["zero", "one"],
            vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 0,
                    sensor_state: 0,
                    mass: half(),
                },
            ],
            |hypothesis, physical, sensor, action| {
                if action == 0 {
                    let next = if (hypothesis == 0 && physical == 0)
                        || (hypothesis == 1 && physical == 1)
                    {
                        0
                    } else {
                        1
                    };
                    vec![
                        JointOutcome {
                            probability: half(),
                            next_physical_state: 0,
                            next_sensor_state: sensor,
                            observation: Some(0),
                        },
                        JointOutcome {
                            probability: half(),
                            next_physical_state: 1,
                            next_sensor_state: sensor,
                            observation: Some(1),
                        },
                    ]
                } else {
                    vec![JointOutcome {
                        probability: Rational::one(),
                        next_physical_state: physical,
                        next_sensor_state: sensor,
                        observation: Some(physical),
                    }]
                }
            },
        )
    }

    fn three_hypothesis_kernel() -> JointKernel {
        make_kernel(
            3,
            1,
            1,
            vec!["read"],
            vec!["zero", "one"],
            vec![
                JointMass { hypothesis: 0, physical_state: 0, sensor_state: 0, mass: one_third() },
                JointMass { hypothesis: 1, physical_state: 0, sensor_state: 0, mass: one_third() },
                JointMass { hypothesis: 2, physical_state: 0, sensor_state: 0, mass: one_third() },
            ],
            |hypothesis, physical, sensor, _action| {
                vec![JointOutcome {
                    probability: Rational::one(),
                    next_physical_state: physical,
                    next_sensor_state: sensor,
                    observation: Some(if hypothesis == 2 { 1 } else { 0 }),
                }]
            },
        )
    }

    #[test]
    fn signed_rational_arithmetic_is_exact() {
        let positive = SignedRational::positive(half());
        let negative = SignedRational::negative(quarter());
        assert_eq!(
            positive.checked_add(negative).unwrap(),
            SignedRational::positive(quarter())
        );
        assert_eq!(
            positive.checked_sub(positive).unwrap(),
            SignedRational::zero()
        );
        assert_eq!(
            negative.checked_mul(negative).unwrap(),
            SignedRational::positive(quarter())
        );
        assert_eq!(
            positive.checked_div(negative).unwrap(),
            SignedRational::negative(Rational::one())
        );
    }

    #[test]
    fn equivalent_hypotheses_with_different_joint_hidden_states_get_all_horizon_certificate() {
        let kernel = equivalent_different_hidden_states();
        let classification =
            analyze_hypothesis_pair(&kernel, 0, 1, &config()).unwrap();
        let PairClassification::Equivalent(certificate) = classification else {
            panic!("expected an all-horizon equivalence certificate");
        };
        assert!(verify_equivalence_certificate(&kernel, &certificate).unwrap());
        assert_eq!(certificate.ambient_dimension, 10);
    }

    #[test]
    fn distinguishing_word_can_require_multiple_experiments() {
        let kernel = multi_step_identifying_kernel();
        let classification =
            analyze_hypothesis_pair(&kernel, 0, 1, &config()).unwrap();
        let PairClassification::Distinguishable(witness) = classification else {
            panic!("expected a concrete distinguishing sequence");
        };
        assert_eq!(witness.steps.len(), 2);
        assert_eq!(witness.steps[0].action_id, "prime");
        assert_eq!(witness.steps[1].action_id, "read");
        assert!(!witness.probability_difference.is_zero());
        assert!(verify_distinguishing_witness(&kernel, &witness).unwrap());
    }

    #[test]
    fn hazard_is_an_observable_and_can_distinguish_hypotheses() {
        let kernel = hazard_identifying_kernel();
        let classification =
            analyze_hypothesis_pair(&kernel, 0, 1, &config()).unwrap();
        let PairClassification::Distinguishable(witness) = classification else {
            panic!("expected hazard distinction");
        };
        assert_eq!(witness.steps.len(), 1);
        assert_eq!(witness.steps[0].outcome, OutcomeSymbol::Hazard);
        assert!(verify_distinguishing_witness(&kernel, &witness).unwrap());
    }

    #[test]
    fn one_step_marginals_can_match_while_two_step_correlations_distinguish() {
        let kernel = correlation_only_kernel();
        let classification =
            analyze_hypothesis_pair(&kernel, 0, 1, &config()).unwrap();
        let PairClassification::Distinguishable(witness) = classification else {
            panic!("expected a two-step correlation distinction");
        };
        assert_eq!(witness.steps.len(), 2);
        assert_eq!(witness.steps[0].action_id, "tap");
        assert_eq!(witness.steps[1].action_id, "read");
        assert!(verify_distinguishing_witness(&kernel, &witness).unwrap());
    }

    #[test]
    fn equivalence_classes_are_complete_and_transitive() {
        let kernel = three_hypothesis_kernel();
        let report = analyze_identifiability(&kernel, &config()).unwrap();
        assert_eq!(report.equivalence_classes, vec![vec!["h0".to_string(), "h1".to_string()], vec!["h2".to_string()]]);
        assert_eq!(report.pairwise.len(), 3);
    }

    #[test]
    fn zero_prior_hypotheses_are_reported_not_silently_classified() {
        let mut kernel = three_hypothesis_kernel();
        kernel.initial_belief.retain(|point| point.hypothesis != 2);
        for point in &mut kernel.initial_belief {
            point.mass = half();
        }
        let report = analyze_identifiability(&kernel, &config()).unwrap();
        assert_eq!(report.supported_hypotheses, vec!["h0".to_string(), "h1".to_string()]);
        assert_eq!(report.zero_prior_hypotheses, vec!["h2".to_string()]);
    }

    #[test]
    fn exact_small_probability_difference_is_never_rounded_to_zero() {
        let epsilon = Rational::new(1, u128::MAX).unwrap();
        let complement = Rational::one().checked_sub(epsilon).unwrap();
        let kernel = make_kernel(
            2,
            1,
            1,
            vec!["precision-test"],
            vec!["zero", "one"],
            vec![
                JointMass { hypothesis: 0, physical_state: 0, sensor_state: 0, mass: half() },
                JointMass { hypothesis: 1, physical_state: 0, sensor_state: 0, mass: half() },
            ],
            |hypothesis, physical, sensor, _action| {
                if hypothesis == 0 {
                    vec![
                        JointOutcome {
                            probability: Rational::one(),
                            next_physical_state: physical,
                            next_sensor_state: sensor,
                            observation: Some(0),
                        },
                    ]
                } else {
                    vec![
                        JointOutcome {
                            probability: complement,
                            next_physical_state: physical,
                            next_sensor_state: sensor,
                            observation: Some(0),
                        },
                        JointOutcome {
                            probability: epsilon,
                            next_physical_state: physical,
                            next_sensor_state: sensor,
                            observation: Some(1),
                        },
                    ]
                }
            },
        );
        let classification = analyze_hypothesis_pair(&kernel, 0, 1, &config()).unwrap();
        let PairClassification::Distinguishable(witness) = classification else {
            panic!("tiny exact difference must distinguish");
        };
        assert_eq!(witness.probability_difference.numerator(), 1);
        assert_eq!(witness.probability_difference.denominator(), u128::MAX);
        assert!(verify_distinguishing_witness(&kernel, &witness).unwrap());
    }

    #[test]
    fn independent_exhaustive_word_enumerator_agrees_with_pairwise_classification() {
        let kernels = vec![
            equivalent_different_hidden_states(),
            multi_step_identifying_kernel(),
            hazard_identifying_kernel(),
            correlation_only_kernel(),
        ];
        for kernel in kernels {
            let report = analyze_identifiability(&kernel, &config()).unwrap();
            for pair in report.pairwise {
                match pair.classification {
                    PairClassification::Distinguishable(witness) => {
                        assert!(verify_distinguishing_witness(&kernel, &witness).unwrap());
                    }
                    PairClassification::Equivalent(certificate) => {
                        assert!(verify_equivalence_certificate(&kernel, &certificate).unwrap());
                    }
                }
            }
        }
    }

    #[test]
    fn resource_caps_fail_closed_and_never_emit_equivalence() {
        let kernel = equivalent_different_hidden_states();
        let limited = IdentifiabilityConfig {
            max_ambient_dimension: 1,
            ..config()
        };
        assert!(analyze_hypothesis_pair(&kernel, 0, 1, &limited).is_err());

        let limited_ops = IdentifiabilityConfig {
            max_transition_applications: 1,
            ..config()
        };
        assert!(analyze_hypothesis_pair(&kernel, 0, 1, &limited_ops).is_err());
    }

    #[test]
    fn unused_exact_probability_helpers_are_well_formed() {
        assert_eq!(three_quarters().checked_sub(one_third()).unwrap(), Rational::new(5, 12).unwrap());
        assert_eq!(two_thirds().checked_mul(three_quarters()).unwrap(), half());
    }
}
