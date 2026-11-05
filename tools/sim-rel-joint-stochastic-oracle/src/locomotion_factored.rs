//! Exact factorized transition inference for locomotion-scale state spaces.
//!
//! This module is deliberately separate from the dense JointKernel oracle.
//! It represents a normalized dynamic Bayesian network over input variables
//! (current state and action) and output variables (next state and observation).
//! Conditional probability tables remain explicit and source-qualified;
//! inference uses exact rational sum-product and variable elimination.
//!
//! The dense oracle remains the independent reference for small models. This
//! module never silently materializes a Cartesian state product.

use crate::Rational;
use crate::locomotion::{ClaimCeiling, EvidenceClass, SourceKind, SourceManifest};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_VARIABLES: usize = 128;
const MAX_DOMAIN_VALUES: usize = 128;
const MAX_PARENTS_PER_FACTOR: usize = 12;
const MAX_PARENT_ASSIGNMENTS_PER_FACTOR: usize = 65_536;
const MAX_TOTAL_CONDITIONAL_ROWS: usize = 262_144;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum VariableRole {
    Input,
    Action,
    Output,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscreteVariable {
    pub id: String,
    pub role: VariableRole,
    pub domain: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorOutcome {
    pub value: String,
    pub probability: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalRow {
    /// Values in the same order as ConditionalFactor::parent_variable_ids.
    pub parent_values: Vec<String>,
    /// Omitted target-domain values have probability zero.
    pub outcomes: Vec<FactorOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalFactor {
    pub id: String,
    pub target_variable_id: String,
    pub parent_variable_ids: Vec<String>,
    pub source_manifest_ids: Vec<String>,
    pub claim_ceiling: ClaimCeiling,
    pub rows: Vec<ConditionalRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterventionScopeContract {
    pub action_variable_id: String,
    pub direct_target_output_ids: Vec<String>,
    pub allowed_descendant_output_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorizedTransitionSpec {
    pub source_manifests: Vec<SourceManifest>,
    pub variables: Vec<DiscreteVariable>,
    pub factors: Vec<ConditionalFactor>,
    pub intervention_scopes: Vec<InterventionScopeContract>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorError {
    Invalid(String),
    UnknownVariable(String),
    UnknownDomainValue { variable_id: String, value: String },
    MissingSource(String),
    DuplicateIdentifier(String),
    CycleDetected,
    MissingInput(String),
    MissingOutput(String),
    InvalidQuery(String),
    Arithmetic(String),
    ResourceLimit { requested: usize, limit: usize },
    ZeroMass,
}

impl fmt::Display for FactorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FactorError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CompiledOutcome {
    value: usize,
    probability: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CompiledRow {
    parent_values: Vec<usize>,
    outcomes: Vec<CompiledOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledConditionalFactor {
    pub id: String,
    pub target_variable: usize,
    pub parent_variables: Vec<usize>,
    pub source_manifest_ids: Vec<String>,
    pub claim_ceiling: ClaimCeiling,
    rows: Vec<CompiledRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledInterventionScope {
    pub action_variable: usize,
    pub direct_targets: Vec<usize>,
    pub allowed_descendants: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledFactorizedTransition {
    pub source_manifests: Vec<SourceManifest>,
    pub variables: Vec<DiscreteVariable>,
    pub factors: Vec<CompiledConditionalFactor>,
    pub intervention_scopes: Vec<CompiledInterventionScope>,
    /// Stable topological order of output variables.
    pub output_order: Vec<usize>,
    pub input_variables: Vec<usize>,
    pub output_variables: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarginalEntry {
    pub values: BTreeMap<String, String>,
    pub probability: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FactorTable {
    /// Sorted global variable indices. All variables here are outputs.
    scope: Vec<usize>,
    /// Sparse exact table; zero-mass entries are not stored.
    entries: BTreeMap<Vec<usize>, Rational>,
}

impl FactorizedTransitionSpec {
    /// Canonicalize IDs and validate all CPDs, source claims and dependencies.
    ///
    /// Parent IDs are sorted semantically and each row's parent assignment is
    /// permuted with them. Variables, factors, rows and outcomes are then
    /// canonicalized so caller storage order cannot change inference results.
    pub fn compile(&self) -> Result<CompiledFactorizedTransition, FactorError> {
        let mut spec = self.clone();
        spec.variables.sort_by(|left, right| left.id.cmp(&right.id));
        spec.source_manifests
            .sort_by(|left, right| left.id.cmp(&right.id));

        for factor in &mut spec.factors {
            let old_parent_ids = factor.parent_variable_ids.clone();
            let mut permutation: Vec<usize> = (0..old_parent_ids.len()).collect();
            permutation.sort_by(|left, right| old_parent_ids[*left].cmp(&old_parent_ids[*right]));

            factor.parent_variable_ids = permutation
                .iter()
                .map(|index| old_parent_ids[*index].clone())
                .collect();

            for row in &mut factor.rows {
                if row.parent_values.len() != permutation.len() {
                    return Err(FactorError::Invalid(format!(
                        "factor {} has a parent row with the wrong arity",
                        factor.id
                    )));
                }
                let old_values = row.parent_values.clone();
                row.parent_values = permutation
                    .iter()
                    .map(|index| old_values[*index].clone())
                    .collect();
                row.outcomes
                    .sort_by(|left, right| left.value.cmp(&right.value));
            }

            factor
                .rows
                .sort_by(|left, right| left.parent_values.cmp(&right.parent_values));
            factor.source_manifest_ids.sort();
        }
        spec.factors
            .sort_by(|left, right| left.target_variable_id.cmp(&right.target_variable_id));
        for scope in &mut spec.intervention_scopes {
            scope.direct_target_output_ids.sort();
            scope.allowed_descendant_output_ids.sort();
        }
        spec.intervention_scopes
            .sort_by(|left, right| left.action_variable_id.cmp(&right.action_variable_id));

        validate_spec_shape(&spec)?;

        let variable_indices: BTreeMap<String, usize> = spec
            .variables
            .iter()
            .enumerate()
            .map(|(index, variable)| (variable.id.clone(), index))
            .collect();

        let source_indices: BTreeMap<String, &SourceManifest> = spec
            .source_manifests
            .iter()
            .map(|source| (source.id.clone(), source))
            .collect();

        let mut factors = Vec::with_capacity(spec.factors.len());
        let mut targeted_variables = BTreeSet::new();
        let mut total_rows = 0_usize;

        for factor in &spec.factors {
            let target_variable = *variable_indices
                .get(&factor.target_variable_id)
                .ok_or_else(|| FactorError::UnknownVariable(factor.target_variable_id.clone()))?;

            if spec.variables[target_variable].role != VariableRole::Output {
                return Err(FactorError::Invalid(format!(
                    "factor {} targets a non-output variable",
                    factor.id
                )));
            }
            if !targeted_variables.insert(target_variable) {
                return Err(FactorError::DuplicateIdentifier(format!(
                    "multiple CPDs target {}",
                    factor.target_variable_id
                )));
            }
            if factor.parent_variable_ids.len() > MAX_PARENTS_PER_FACTOR {
                return Err(FactorError::ResourceLimit {
                    requested: factor.parent_variable_ids.len(),
                    limit: MAX_PARENTS_PER_FACTOR,
                });
            }
            if factor.source_manifest_ids.is_empty() {
                return Err(FactorError::MissingSource(factor.id.clone()));
            }

            let mut parent_variables = Vec::with_capacity(factor.parent_variable_ids.len());
            let mut expected_parent_assignments = 1_usize;
            for parent_id in &factor.parent_variable_ids {
                let parent = *variable_indices
                    .get(parent_id)
                    .ok_or_else(|| FactorError::UnknownVariable(parent_id.clone()))?;
                if parent == target_variable {
                    return Err(FactorError::Invalid(format!(
                        "factor {} depends on its own target",
                        factor.id
                    )));
                }
                parent_variables.push(parent);
                expected_parent_assignments = expected_parent_assignments
                    .checked_mul(spec.variables[parent].domain.len())
                    .ok_or_else(|| FactorError::ResourceLimit {
                        requested: usize::MAX,
                        limit: MAX_PARENT_ASSIGNMENTS_PER_FACTOR,
                    })?;
            }
            if expected_parent_assignments > MAX_PARENT_ASSIGNMENTS_PER_FACTOR {
                return Err(FactorError::ResourceLimit {
                    requested: expected_parent_assignments,
                    limit: MAX_PARENT_ASSIGNMENTS_PER_FACTOR,
                });
            }
            if factor.rows.len() != expected_parent_assignments {
                return Err(FactorError::Invalid(format!(
                    "factor {} has {} parent rows; expected {}",
                    factor.id,
                    factor.rows.len(),
                    expected_parent_assignments
                )));
            }

            let mut seen_parent_rows = BTreeSet::new();
            let mut compiled_rows = Vec::with_capacity(factor.rows.len());
            for row in &factor.rows {
                if row.parent_values.len() != parent_variables.len() {
                    return Err(FactorError::Invalid(format!(
                        "factor {} has a malformed parent assignment",
                        factor.id
                    )));
                }

                let mut parent_value_indices = Vec::with_capacity(parent_variables.len());
                for (position, value) in row.parent_values.iter().enumerate() {
                    let parent = parent_variables[position];
                    let index = domain_index(&spec.variables[parent], value)?;
                    parent_value_indices.push(index);
                }
                if !seen_parent_rows.insert(parent_value_indices.clone()) {
                    return Err(FactorError::DuplicateIdentifier(format!(
                        "factor {} repeats a parent assignment",
                        factor.id
                    )));
                }

                if row.outcomes.is_empty() {
                    return Err(FactorError::Invalid(format!(
                        "factor {} has an empty conditional outcome row",
                        factor.id
                    )));
                }
                let target_domain = &spec.variables[target_variable].domain;
                let mut outcome_values = BTreeSet::new();
                let mut mass = Rational::zero();
                let mut compiled_outcomes = Vec::with_capacity(row.outcomes.len());
                for outcome in &row.outcomes {
                    if outcome.probability.is_zero() || !outcome.probability.is_probability() {
                        return Err(FactorError::Invalid(format!(
                            "factor {} contains a non-positive or invalid probability",
                            factor.id
                        )));
                    }
                    if !outcome_values.insert(outcome.value.clone()) {
                        return Err(FactorError::DuplicateIdentifier(format!(
                            "factor {} repeats target outcome {}",
                            factor.id, outcome.value
                        )));
                    }
                    let value_index = target_domain
                        .iter()
                        .position(|candidate| candidate == &outcome.value)
                        .ok_or_else(|| FactorError::UnknownDomainValue {
                            variable_id: spec.variables[target_variable].id.clone(),
                            value: outcome.value.clone(),
                        })?;
                    mass = mass
                        .checked_add(outcome.probability)
                        .map_err(FactorError::Arithmetic)?;
                    compiled_outcomes.push(CompiledOutcome {
                        value: value_index,
                        probability: outcome.probability,
                    });
                }
                if mass != Rational::one() {
                    return Err(FactorError::Invalid(format!(
                        "factor {} conditional row mass is not exactly one",
                        factor.id
                    )));
                }
                compiled_rows.push(CompiledRow {
                    parent_values: parent_value_indices,
                    outcomes: compiled_outcomes,
                });
            }

            if seen_parent_rows.len() != expected_parent_assignments {
                return Err(FactorError::Invalid(format!(
                    "factor {} does not cover its complete parent domain",
                    factor.id
                )));
            }

            let mut unique_sources = BTreeSet::new();
            for source_id in &factor.source_manifest_ids {
                if !unique_sources.insert(source_id.clone()) {
                    return Err(FactorError::DuplicateIdentifier(format!(
                        "factor {} repeats source {}",
                        factor.id, source_id
                    )));
                }
                let source = source_indices
                    .get(source_id)
                    .ok_or_else(|| FactorError::MissingSource(source_id.clone()))?;
                if !source_supports_claim(source, factor.claim_ceiling) {
                    return Err(FactorError::Invalid(format!(
                        "factor {} claim ceiling is not supported by source {}",
                        factor.id, source_id
                    )));
                }
            }

            total_rows = total_rows.checked_add(compiled_rows.len()).ok_or_else(|| {
                FactorError::ResourceLimit {
                    requested: usize::MAX,
                    limit: MAX_TOTAL_CONDITIONAL_ROWS,
                }
            })?;
            if total_rows > MAX_TOTAL_CONDITIONAL_ROWS {
                return Err(FactorError::ResourceLimit {
                    requested: total_rows,
                    limit: MAX_TOTAL_CONDITIONAL_ROWS,
                });
            }

            factors.push(CompiledConditionalFactor {
                id: factor.id.clone(),
                target_variable,
                parent_variables,
                source_manifest_ids: factor.source_manifest_ids.clone(),
                claim_ceiling: factor.claim_ceiling,
                rows: compiled_rows,
            });
        }

        let output_variables: Vec<usize> = spec
            .variables
            .iter()
            .enumerate()
            .filter_map(|(index, variable)| {
                (variable.role == VariableRole::Output).then_some(index)
            })
            .collect();
        if targeted_variables.len() != output_variables.len() {
            let missing: Vec<String> = output_variables
                .iter()
                .filter(|index| !targeted_variables.contains(index))
                .map(|index| spec.variables[*index].id.clone())
                .collect();
            return Err(FactorError::Invalid(format!(
                "output variables have no CPD: {}",
                missing.join(", ")
            )));
        }

        let output_order = topological_output_order(&spec.variables, &factors)?;
        let intervention_scopes =
            compile_intervention_scopes(&spec.variables, &factors, &spec.intervention_scopes)?;
        let input_variables = spec
            .variables
            .iter()
            .enumerate()
            .filter_map(|(index, variable)| {
                (variable.role != VariableRole::Output).then_some(index)
            })
            .collect();

        Ok(CompiledFactorizedTransition {
            source_manifests: spec.source_manifests,
            variables: spec.variables,
            factors,
            intervention_scopes,
            output_order,
            input_variables,
            output_variables,
        })
    }
}

impl CompiledFactorizedTransition {
    pub fn dense_output_cardinality(&self) -> Result<usize, FactorError> {
        checked_domain_product(
            self.output_variables
                .iter()
                .map(|index| self.variables[*index].domain.len()),
        )
    }

    /// Exact joint probability for one complete input/output assignment.
    pub fn joint_probability(
        &self,
        inputs: &BTreeMap<String, String>,
        outputs: &BTreeMap<String, String>,
    ) -> Result<Rational, FactorError> {
        let input_values = self.validate_input_assignment(inputs)?;
        let output_values = self.validate_output_assignment(outputs)?;
        let mut probability = Rational::one();

        for factor in &self.factors {
            let parent_values: Vec<usize> = factor
                .parent_variables
                .iter()
                .map(|parent| {
                    if self.variables[*parent].role != VariableRole::Output {
                        Ok(input_values[parent])
                    } else {
                        Ok(output_values[parent])
                    }
                })
                .collect::<Result<_, FactorError>>()?;

            let row = factor
                .rows
                .iter()
                .find(|row| row.parent_values == parent_values)
                .ok_or_else(|| {
                    FactorError::Invalid(format!(
                        "compiled factor {} is missing a parent row",
                        factor.id
                    ))
                })?;
            let target_value = output_values[&factor.target_variable];
            let conditional_probability = row
                .outcomes
                .iter()
                .find(|outcome| outcome.value == target_value)
                .map(|outcome| outcome.probability)
                .unwrap_or_else(Rational::zero);
            probability = probability
                .checked_mul(conditional_probability)
                .map_err(FactorError::Arithmetic)?;
            if probability.is_zero() {
                break;
            }
        }
        Ok(probability)
    }

    /// Exact marginal by variable elimination.
    ///
    /// The input assignment fixes current state/action variables. The query
    /// contains any subset of output variables. Intermediate tables and the
    /// returned query table are bounded by max_intermediate_rows; the complete
    /// Cartesian output state is never constructed unless it is itself queried.
    pub fn marginal(
        &self,
        inputs: &BTreeMap<String, String>,
        query_output_ids: &[String],
        max_intermediate_rows: usize,
    ) -> Result<Vec<MarginalEntry>, FactorError> {
        if max_intermediate_rows == 0 {
            return Err(FactorError::Invalid(
                "max_intermediate_rows must be positive".to_owned(),
            ));
        }
        let input_values = self.validate_input_assignment(inputs)?;
        let query_indices = self.validate_query(query_output_ids)?;
        let requested_rows = checked_domain_product(
            query_indices
                .iter()
                .map(|index| self.variables[*index].domain.len()),
        )?;
        if requested_rows > max_intermediate_rows {
            return Err(FactorError::ResourceLimit {
                requested: requested_rows,
                limit: max_intermediate_rows,
            });
        }

        let mut work = Vec::with_capacity(self.factors.len());
        for factor in &self.factors {
            let mut scope: Vec<usize> = factor
                .parent_variables
                .iter()
                .copied()
                .filter(|parent| self.variables[*parent].role == VariableRole::Output)
                .chain([factor.target_variable])
                .collect();
            scope.sort_unstable();
            scope.dedup();

            let mut entries = BTreeMap::new();
            for row in &factor.rows {
                let input_matches = factor
                    .parent_variables
                    .iter()
                    .enumerate()
                    .filter(|(_, parent)| self.variables[**parent].role != VariableRole::Output)
                    .all(|(position, parent)| row.parent_values[position] == input_values[parent]);
                if !input_matches {
                    continue;
                }

                for outcome in &row.outcomes {
                    let mut assignment = Vec::with_capacity(scope.len());
                    for variable in &scope {
                        if *variable == factor.target_variable {
                            assignment.push(outcome.value);
                        } else {
                            let parent_position = factor
                                .parent_variables
                                .iter()
                                .position(|parent| parent == variable)
                                .ok_or_else(|| {
                                    FactorError::Invalid(format!(
                                        "factor {} has an inconsistent scope",
                                        factor.id
                                    ))
                                })?;
                            assignment.push(row.parent_values[parent_position]);
                        }
                    }
                    entries.insert(assignment, outcome.probability);
                }
            }
            if entries.is_empty() {
                return Err(FactorError::Invalid(format!(
                    "factor {} has no rows for the supplied input assignment",
                    factor.id
                )));
            }
            work.push(FactorTable { scope, entries });
        }

        let query_set: BTreeSet<usize> = query_indices.iter().copied().collect();
        let mut hidden: BTreeSet<usize> = self
            .output_variables
            .iter()
            .copied()
            .filter(|index| !query_set.contains(index))
            .collect();

        while !hidden.is_empty() {
            let variable = choose_elimination_variable(&work, &hidden, &self.variables)?;
            let mut bucket = Vec::new();
            let mut retained = Vec::new();
            for factor in work {
                if factor.scope.contains(&variable) {
                    bucket.push(factor);
                } else {
                    retained.push(factor);
                }
            }
            if bucket.is_empty() {
                return Err(FactorError::Invalid(format!(
                    "output variable {} has no active factor",
                    self.variables[variable].id
                )));
            }

            let mut product = bucket.remove(0);
            for factor in bucket {
                product =
                    multiply_tables(&product, &factor, &self.variables, max_intermediate_rows)?;
            }
            let reduced = sum_out_variable(&product, variable)?;
            if reduced.entries.len() > max_intermediate_rows {
                return Err(FactorError::ResourceLimit {
                    requested: reduced.entries.len(),
                    limit: max_intermediate_rows,
                });
            }
            retained.push(reduced);
            work = retained;
            hidden.remove(&variable);
        }

        if work.is_empty() {
            return Ok(vec![MarginalEntry {
                values: BTreeMap::new(),
                probability: Rational::one(),
            }]);
        }

        let mut product = work.remove(0);
        for factor in work {
            product = multiply_tables(&product, &factor, &self.variables, max_intermediate_rows)?;
        }

        if product.scope.iter().any(|index| !query_set.contains(index))
            || product.scope.len() != query_set.len()
        {
            return Err(FactorError::Invalid(
                "variable elimination left a non-query variable in the result".to_owned(),
            ));
        }

        let mut entries = Vec::with_capacity(product.entries.len());
        let mut total_mass = Rational::zero();
        for (assignment, probability) in &product.entries {
            let mut values = BTreeMap::new();
            for (position, variable) in product.scope.iter().enumerate() {
                let domain_value = self.variables[*variable]
                    .domain
                    .get(assignment[position])
                    .ok_or_else(|| {
                        FactorError::Invalid("factor table contains an invalid value".to_owned())
                    })?;
                values.insert(self.variables[*variable].id.clone(), domain_value.clone());
            }
            total_mass = total_mass
                .checked_add(*probability)
                .map_err(FactorError::Arithmetic)?;
            entries.push(MarginalEntry {
                values,
                probability: *probability,
            });
        }

        if total_mass.is_zero() {
            return Err(FactorError::ZeroMass);
        }
        if total_mass != Rational::one() {
            return Err(FactorError::Invalid(
                "factorized marginal does not conserve exact probability mass".to_owned(),
            ));
        }
        entries.sort_by(|left, right| left.values.cmp(&right.values));
        Ok(entries)
    }

    /// Explicit dense output enumeration, guarded before allocation.
    pub fn enumerate_dense_outputs(
        &self,
        inputs: &BTreeMap<String, String>,
        max_assignments: usize,
    ) -> Result<Vec<MarginalEntry>, FactorError> {
        let cardinality = self.dense_output_cardinality()?;
        if cardinality > max_assignments {
            return Err(FactorError::ResourceLimit {
                requested: cardinality,
                limit: max_assignments,
            });
        }
        let query: Vec<String> = self
            .output_variables
            .iter()
            .map(|index| self.variables[*index].id.clone())
            .collect();
        self.marginal(inputs, &query, max_assignments)
    }

    fn validate_input_assignment(
        &self,
        inputs: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<usize, usize>, FactorError> {
        if inputs.len() != self.input_variables.len() {
            return Err(FactorError::Invalid(
                "input assignment must contain every input variable exactly once".to_owned(),
            ));
        }
        let mut values = BTreeMap::new();
        for index in &self.input_variables {
            let variable = &self.variables[*index];
            let value = inputs
                .get(&variable.id)
                .ok_or_else(|| FactorError::MissingInput(variable.id.clone()))?;
            values.insert(*index, domain_index(variable, value)?);
        }
        for key in inputs.keys() {
            if !self
                .variables
                .iter()
                .any(|variable| variable.role != VariableRole::Output && variable.id == *key)
            {
                return Err(FactorError::Invalid(format!(
                    "input assignment contains non-input variable {key}"
                )));
            }
        }
        Ok(values)
    }

    fn validate_output_assignment(
        &self,
        outputs: &BTreeMap<String, String>,
    ) -> Result<BTreeMap<usize, usize>, FactorError> {
        if outputs.len() != self.output_variables.len() {
            return Err(FactorError::Invalid(
                "output assignment must contain every output variable exactly once".to_owned(),
            ));
        }
        let mut values = BTreeMap::new();
        for index in &self.output_variables {
            let variable = &self.variables[*index];
            let value = outputs
                .get(&variable.id)
                .ok_or_else(|| FactorError::MissingOutput(variable.id.clone()))?;
            values.insert(*index, domain_index(variable, value)?);
        }
        for key in outputs.keys() {
            if !self
                .variables
                .iter()
                .any(|variable| variable.role == VariableRole::Output && variable.id == *key)
            {
                return Err(FactorError::Invalid(format!(
                    "output assignment contains non-output variable {key}"
                )));
            }
        }
        Ok(values)
    }

    fn validate_query(&self, query_ids: &[String]) -> Result<Vec<usize>, FactorError> {
        let mut indices = Vec::with_capacity(query_ids.len());
        let mut seen = BTreeSet::new();
        for id in query_ids {
            let index = self
                .variables
                .iter()
                .position(|variable| variable.id == *id)
                .ok_or_else(|| FactorError::UnknownVariable(id.clone()))?;
            if self.variables[index].role != VariableRole::Output {
                return Err(FactorError::InvalidQuery(format!(
                    "query variable {id} is not an output"
                )));
            }
            if !seen.insert(index) {
                return Err(FactorError::InvalidQuery(format!(
                    "query repeats output variable {id}"
                )));
            }
            indices.push(index);
        }
        indices.sort_unstable();
        Ok(indices)
    }
}

fn validate_spec_shape(spec: &FactorizedTransitionSpec) -> Result<(), FactorError> {
    if spec.variables.is_empty() || spec.variables.len() > MAX_VARIABLES {
        return Err(FactorError::ResourceLimit {
            requested: spec.variables.len(),
            limit: MAX_VARIABLES,
        });
    }
    if spec.factors.is_empty() {
        return Err(FactorError::Invalid(
            "factorized transition requires at least one conditional factor".to_owned(),
        ));
    }

    let mut variable_ids = BTreeSet::new();
    for variable in &spec.variables {
        if variable.id.trim().is_empty() || !variable_ids.insert(variable.id.clone()) {
            return Err(FactorError::DuplicateIdentifier(variable.id.clone()));
        }
        if variable.domain.is_empty() || variable.domain.len() > MAX_DOMAIN_VALUES {
            return Err(FactorError::ResourceLimit {
                requested: variable.domain.len(),
                limit: MAX_DOMAIN_VALUES,
            });
        }
        let mut domain_values = BTreeSet::new();
        for value in &variable.domain {
            if value.trim().is_empty() || !domain_values.insert(value.clone()) {
                return Err(FactorError::DuplicateIdentifier(format!(
                    "{}:{}",
                    variable.id, value
                )));
            }
        }
    }

    let mut source_ids = BTreeSet::new();
    for source in &spec.source_manifests {
        if source.id.trim().is_empty() || !source_ids.insert(source.id.clone()) {
            return Err(FactorError::DuplicateIdentifier(source.id.clone()));
        }
    }

    let mut factor_ids = BTreeSet::new();
    let mut target_ids = BTreeSet::new();
    for factor in &spec.factors {
        if factor.id.trim().is_empty() || !factor_ids.insert(factor.id.clone()) {
            return Err(FactorError::DuplicateIdentifier(factor.id.clone()));
        }
        if !target_ids.insert(factor.target_variable_id.clone()) {
            return Err(FactorError::DuplicateIdentifier(format!(
                "target:{}",
                factor.target_variable_id
            )));
        }
        if factor.parent_variable_ids.len() > MAX_PARENTS_PER_FACTOR {
            return Err(FactorError::ResourceLimit {
                requested: factor.parent_variable_ids.len(),
                limit: MAX_PARENTS_PER_FACTOR,
            });
        }
        let mut parents = BTreeSet::new();
        for parent in &factor.parent_variable_ids {
            if !parents.insert(parent.clone()) {
                return Err(FactorError::DuplicateIdentifier(format!(
                    "parent:{}:{}",
                    factor.id, parent
                )));
            }
        }
    }
    Ok(())
}

fn domain_index(variable: &DiscreteVariable, value: &str) -> Result<usize, FactorError> {
    variable
        .domain
        .iter()
        .position(|candidate| candidate == value)
        .ok_or_else(|| FactorError::UnknownDomainValue {
            variable_id: variable.id.clone(),
            value: value.to_owned(),
        })
}

fn source_supports_claim(source: &SourceManifest, claim: ClaimCeiling) -> bool {
    match claim {
        ClaimCeiling::SyntheticQualification => {
            source.evidence_class == EvidenceClass::SyntheticFixture
        }
        ClaimCeiling::InSilicoReproduction => {
            source.kind == SourceKind::SimulationCode
                && source.evidence_class == EvidenceClass::InSilicoGenerated
        }
        ClaimCeiling::AnatomicalConnectomeConstrained => {
            source.kind == SourceKind::Connectome
                && source.evidence_class == EvidenceClass::ConnectomeDerived
        }
        ClaimCeiling::PreprintMeasuredNeuralReproduction => {
            source.kind == SourceKind::NeuralMeasurement
                && source.evidence_class == EvidenceClass::PreprintMeasured
        }
        ClaimCeiling::PeerReviewedMeasuredNeuralReproduction => {
            source.kind == SourceKind::NeuralMeasurement
                && source.evidence_class == EvidenceClass::PeerReviewedMeasured
        }
        ClaimCeiling::PeerReviewedMeasuredBehaviourReproduction => {
            matches!(
                source.kind,
                SourceKind::KinematicMeasurement | SourceKind::BehaviouralMeasurement
            ) && source.evidence_class == EvidenceClass::PeerReviewedMeasured
        }
    }
}

fn topological_output_order(
    variables: &[DiscreteVariable],
    factors: &[CompiledConditionalFactor],
) -> Result<Vec<usize>, FactorError> {
    let output_set: BTreeSet<usize> = variables
        .iter()
        .enumerate()
        .filter_map(|(index, variable)| (variable.role == VariableRole::Output).then_some(index))
        .collect();
    let mut indegree: BTreeMap<usize, usize> = output_set.iter().map(|index| (*index, 0)).collect();
    let mut children: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();

    for factor in factors {
        for parent in &factor.parent_variables {
            if output_set.contains(parent) {
                children
                    .entry(*parent)
                    .or_default()
                    .insert(factor.target_variable);
                *indegree.get_mut(&factor.target_variable).ok_or_else(|| {
                    FactorError::Invalid("factor target is not an output".to_owned())
                })? += 1;
            }
        }
    }

    let mut ready: BTreeSet<usize> = indegree
        .iter()
        .filter_map(|(variable, degree)| (*degree == 0).then_some(*variable))
        .collect();
    let mut order = Vec::with_capacity(output_set.len());

    while let Some(variable) = ready.pop_first() {
        order.push(variable);
        if let Some(next_variables) = children.get(&variable) {
            for child in next_variables {
                let degree = indegree.get_mut(child).ok_or_else(|| {
                    FactorError::Invalid("factor graph contains an invalid child".to_owned())
                })?;
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(*child);
                }
            }
        }
    }

    if order.len() != output_set.len() {
        return Err(FactorError::CycleDetected);
    }
    Ok(order)
}

fn compile_intervention_scopes(
    variables: &[DiscreteVariable],
    factors: &[CompiledConditionalFactor],
    contracts: &[InterventionScopeContract],
) -> Result<Vec<CompiledInterventionScope>, FactorError> {
    let variable_indices: BTreeMap<String, usize> = variables
        .iter()
        .enumerate()
        .map(|(index, variable)| (variable.id.clone(), index))
        .collect();
    let output_indices: BTreeSet<usize> = variables
        .iter()
        .enumerate()
        .filter_map(|(index, variable)| (variable.role == VariableRole::Output).then_some(index))
        .collect();

    let mut contract_by_action = BTreeMap::new();
    for contract in contracts {
        let action_index = *variable_indices
            .get(&contract.action_variable_id)
            .ok_or_else(|| FactorError::UnknownVariable(contract.action_variable_id.clone()))?;
        if variables[action_index].role != VariableRole::Action {
            return Err(FactorError::Invalid(format!(
                "intervention scope {} does not reference an action variable",
                contract.action_variable_id
            )));
        }
        if contract_by_action.insert(action_index, contract).is_some() {
            return Err(FactorError::DuplicateIdentifier(format!(
                "intervention scope:{}",
                contract.action_variable_id
            )));
        }
    }

    let mut children: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let mut direct_action_targets: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for factor in factors {
        for parent in &factor.parent_variables {
            if variables[*parent].role == VariableRole::Action {
                direct_action_targets
                    .entry(*parent)
                    .or_default()
                    .insert(factor.target_variable);
            } else if output_indices.contains(parent) {
                children
                    .entry(*parent)
                    .or_default()
                    .insert(factor.target_variable);
            }
        }
    }

    let action_variables: Vec<usize> = variables
        .iter()
        .enumerate()
        .filter_map(|(index, variable)| (variable.role == VariableRole::Action).then_some(index))
        .collect();
    let mut compiled = Vec::with_capacity(action_variables.len());

    for action in action_variables {
        let contract = contract_by_action.get(&action).ok_or_else(|| {
            FactorError::Invalid(format!(
                "action variable {} has no intervention-scope contract",
                variables[action].id
            ))
        })?;
        let actual_targets = direct_action_targets
            .get(&action)
            .cloned()
            .unwrap_or_default();
        if actual_targets.is_empty() {
            return Err(FactorError::Invalid(format!(
                "action variable {} does not directly target an output factor",
                variables[action].id
            )));
        }

        let declared_targets: BTreeSet<usize> = contract
            .direct_target_output_ids
            .iter()
            .map(|id| {
                let index = *variable_indices
                    .get(id)
                    .ok_or_else(|| FactorError::UnknownVariable(id.clone()))?;
                if !output_indices.contains(&index) {
                    return Err(FactorError::Invalid(format!(
                        "direct target {} is not an output variable",
                        id
                    )));
                }
                Ok(index)
            })
            .collect::<Result<_, FactorError>>()?;
        if declared_targets.len() != contract.direct_target_output_ids.len()
            || declared_targets != actual_targets
        {
            return Err(FactorError::Invalid(format!(
                "action {} directly affects outputs not matching its declared target set",
                variables[action].id
            )));
        }

        let mut closure = actual_targets.clone();
        let mut pending: Vec<usize> = actual_targets.iter().copied().collect();
        while let Some(parent) = pending.pop() {
            if let Some(next) = children.get(&parent) {
                for child in next {
                    if closure.insert(*child) {
                        pending.push(*child);
                    }
                }
            }
        }

        let declared_descendants: BTreeSet<usize> = contract
            .allowed_descendant_output_ids
            .iter()
            .map(|id| {
                let index = *variable_indices
                    .get(id)
                    .ok_or_else(|| FactorError::UnknownVariable(id.clone()))?;
                if !output_indices.contains(&index) {
                    return Err(FactorError::Invalid(format!(
                        "allowed descendant {} is not an output variable",
                        id
                    )));
                }
                Ok(index)
            })
            .collect::<Result<_, FactorError>>()?;
        if declared_descendants.len() != contract.allowed_descendant_output_ids.len()
            || declared_descendants != closure
        {
            return Err(FactorError::Invalid(format!(
                "action {} has hidden or over-declared downstream effects",
                variables[action].id
            )));
        }

        compiled.push(CompiledInterventionScope {
            action_variable: action,
            direct_targets: actual_targets.into_iter().collect(),
            allowed_descendants: closure.into_iter().collect(),
        });
    }

    Ok(compiled)
}

fn checked_domain_product(values: impl Iterator<Item = usize>) -> Result<usize, FactorError> {
    values.try_fold(1_usize, |product, value| {
        product
            .checked_mul(value)
            .ok_or(FactorError::ResourceLimit {
                requested: usize::MAX,
                limit: usize::MAX - 1,
            })
    })
}

fn choose_elimination_variable(
    factors: &[FactorTable],
    candidates: &BTreeSet<usize>,
    variables: &[DiscreteVariable],
) -> Result<usize, FactorError> {
    candidates
        .iter()
        .map(|candidate| {
            let scope: BTreeSet<usize> = factors
                .iter()
                .filter(|factor| factor.scope.contains(candidate))
                .flat_map(|factor| factor.scope.iter().copied())
                .filter(|variable| variable != candidate)
                .collect();
            let width = checked_domain_product(
                scope
                    .iter()
                    .map(|variable| variables[*variable].domain.len()),
            )?;
            Ok((*candidate, width))
        })
        .collect::<Result<Vec<_>, FactorError>>()?
        .into_iter()
        .min_by_key(|(variable, width)| (*width, *variable))
        .map(|(variable, _)| variable)
        .ok_or_else(|| FactorError::Invalid("no elimination candidate remains".to_owned()))
}

fn multiply_tables(
    left: &FactorTable,
    right: &FactorTable,
    variables: &[DiscreteVariable],
    max_rows: usize,
) -> Result<FactorTable, FactorError> {
    let scope: Vec<usize> = left
        .scope
        .iter()
        .chain(&right.scope)
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    let left_positions: Vec<usize> = left
        .scope
        .iter()
        .map(|variable| {
            scope
                .binary_search(variable)
                .expect("scope contains left variable")
        })
        .collect();
    let right_positions: Vec<usize> = right
        .scope
        .iter()
        .map(|variable| {
            scope
                .binary_search(variable)
                .expect("scope contains right variable")
        })
        .collect();

    let mut entries = BTreeMap::new();
    for (left_assignment, left_probability) in &left.entries {
        for (right_assignment, right_probability) in &right.entries {
            let mut merged = vec![None; scope.len()];
            for (position, value) in left_positions.iter().zip(left_assignment) {
                merged[*position] = Some(*value);
            }
            let mut compatible = true;
            for (position, value) in right_positions.iter().zip(right_assignment) {
                match merged[*position] {
                    Some(existing) if existing != *value => {
                        compatible = false;
                        break;
                    }
                    _ => merged[*position] = Some(*value),
                }
            }
            if !compatible {
                continue;
            }

            let assignment: Vec<usize> = merged
                .into_iter()
                .map(|value| {
                    value.ok_or_else(|| {
                        FactorError::Invalid(
                            "factor product left an unassigned variable".to_owned(),
                        )
                    })
                })
                .collect::<Result<_, _>>()?;
            let probability = left_probability
                .checked_mul(*right_probability)
                .map_err(FactorError::Arithmetic)?;
            if probability.is_zero() {
                continue;
            }
            if let Some(existing) = entries.get_mut(&assignment) {
                *existing = existing
                    .checked_add(probability)
                    .map_err(FactorError::Arithmetic)?;
            } else {
                entries.insert(assignment, probability);
                if entries.len() > max_rows {
                    return Err(FactorError::ResourceLimit {
                        requested: entries.len(),
                        limit: max_rows,
                    });
                }
            }
        }
    }

    for (position, variable) in scope.iter().enumerate() {
        if entries
            .keys()
            .any(|assignment| assignment[position] >= variables[*variable].domain.len())
        {
            return Err(FactorError::Invalid(
                "factor product contains a value outside its domain".to_owned(),
            ));
        }
    }
    Ok(FactorTable { scope, entries })
}

fn sum_out_variable(factor: &FactorTable, variable: usize) -> Result<FactorTable, FactorError> {
    let position = factor
        .scope
        .iter()
        .position(|candidate| *candidate == variable)
        .ok_or_else(|| FactorError::UnknownVariable(variable.to_string()))?;
    let scope: Vec<usize> = factor
        .scope
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (index != position).then_some(*value))
        .collect();
    let mut entries = BTreeMap::new();

    for (assignment, probability) in &factor.entries {
        let reduced: Vec<usize> = assignment
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (index != position).then_some(*value))
            .collect();
        if let Some(existing) = entries.get_mut(&reduced) {
            *existing = existing
                .checked_add(*probability)
                .map_err(FactorError::Arithmetic)?;
        } else {
            entries.insert(reduced, *probability);
        }
    }
    Ok(FactorTable { scope, entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_source() -> SourceManifest {
        SourceManifest {
            id: "synthetic-factor-fixture".to_owned(),
            kind: SourceKind::SyntheticFixture,
            evidence_class: EvidenceClass::SyntheticFixture,
            citation_or_source_uri: "synthetic://locomotion/factor-fixture".to_owned(),
            dataset_id: None,
            release_or_materialization: None,
            sha256: None,
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: vec![],
        }
    }

    fn outcome(value: &str, numerator: u128, denominator: u128) -> FactorOutcome {
        FactorOutcome {
            value: value.to_owned(),
            probability: Rational::new(numerator, denominator).unwrap(),
        }
    }

    fn conditional_row(parent_values: &[&str], outcomes: Vec<FactorOutcome>) -> ConditionalRow {
        ConditionalRow {
            parent_values: parent_values
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            outcomes,
        }
    }

    fn factor(
        id: &str,
        target: &str,
        parents: &[&str],
        rows: Vec<ConditionalRow>,
    ) -> ConditionalFactor {
        ConditionalFactor {
            id: id.to_owned(),
            target_variable_id: target.to_owned(),
            parent_variable_ids: parents.iter().map(|value| (*value).to_owned()).collect(),
            source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
            claim_ceiling: ClaimCeiling::SyntheticQualification,
            rows,
        }
    }

    fn spec() -> FactorizedTransitionSpec {
        FactorizedTransitionSpec {
            source_manifests: vec![fixture_source()],
            variables: vec![
                DiscreteVariable {
                    id: "context".to_owned(),
                    role: VariableRole::Input,
                    domain: vec!["walking".to_owned(), "grooming".to_owned()],
                },
                DiscreteVariable {
                    id: "left_leg_state".to_owned(),
                    role: VariableRole::Output,
                    domain: vec!["off".to_owned(), "on".to_owned()],
                },
                DiscreteVariable {
                    id: "right_leg_state".to_owned(),
                    role: VariableRole::Output,
                    domain: vec!["off".to_owned(), "on".to_owned()],
                },
            ],
            factors: vec![
                factor(
                    "left-leg-local-transition",
                    "left_leg_state",
                    &["context"],
                    vec![
                        conditional_row(
                            &["walking"],
                            vec![outcome("off", 1, 4), outcome("on", 3, 4)],
                        ),
                        conditional_row(
                            &["grooming"],
                            vec![outcome("off", 1, 2), outcome("on", 1, 2)],
                        ),
                    ],
                ),
                factor(
                    "right-leg-coupling-transition",
                    "right_leg_state",
                    &["context", "left_leg_state"],
                    vec![
                        conditional_row(
                            &["walking", "off"],
                            vec![outcome("off", 9, 10), outcome("on", 1, 10)],
                        ),
                        conditional_row(
                            &["walking", "on"],
                            vec![outcome("off", 1, 10), outcome("on", 9, 10)],
                        ),
                        conditional_row(
                            &["grooming", "off"],
                            vec![outcome("off", 3, 4), outcome("on", 1, 4)],
                        ),
                        conditional_row(
                            &["grooming", "on"],
                            vec![outcome("off", 1, 4), outcome("on", 3, 4)],
                        ),
                    ],
                ),
            ],
            intervention_scopes: vec![],
        }
    }

    fn scoped_action_spec() -> FactorizedTransitionSpec {
        FactorizedTransitionSpec {
            source_manifests: vec![fixture_source()],
            variables: vec![
                DiscreteVariable {
                    id: "action".to_owned(),
                    role: VariableRole::Action,
                    domain: vec!["activate_left".to_owned(), "idle".to_owned()],
                },
                DiscreteVariable {
                    id: "left_leg_state".to_owned(),
                    role: VariableRole::Output,
                    domain: vec!["off".to_owned(), "on".to_owned()],
                },
                DiscreteVariable {
                    id: "right_leg_state".to_owned(),
                    role: VariableRole::Output,
                    domain: vec!["off".to_owned(), "on".to_owned()],
                },
            ],
            factors: vec![
                factor(
                    "left-leg-action-target",
                    "left_leg_state",
                    &["action"],
                    vec![
                        conditional_row(&["activate_left"], vec![outcome("on", 1, 1)]),
                        conditional_row(&["idle"], vec![outcome("off", 1, 1)]),
                    ],
                ),
                factor(
                    "right-leg-coupling",
                    "right_leg_state",
                    &["left_leg_state"],
                    vec![
                        conditional_row(&["off"], vec![outcome("off", 1, 1)]),
                        conditional_row(&["on"], vec![outcome("off", 1, 10), outcome("on", 9, 10)]),
                    ],
                ),
            ],
            intervention_scopes: vec![InterventionScopeContract {
                action_variable_id: "action".to_owned(),
                direct_target_output_ids: vec!["left_leg_state".to_owned()],
                allowed_descendant_output_ids: vec![
                    "left_leg_state".to_owned(),
                    "right_leg_state".to_owned(),
                ],
            }],
        }
    }

    fn input(context: &str) -> BTreeMap<String, String> {
        BTreeMap::from([("context".to_owned(), context.to_owned())])
    }

    fn output(left: &str, right: &str) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("left_leg_state".to_owned(), left.to_owned()),
            ("right_leg_state".to_owned(), right.to_owned()),
        ])
    }

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    #[test]
    fn exact_marginal_matches_independent_dense_joint_enumeration() {
        let compiled = spec().compile().unwrap();
        let factorized = compiled
            .marginal(
                &input("walking"),
                &["left_leg_state".to_owned(), "right_leg_state".to_owned()],
                16,
            )
            .unwrap();

        let mut dense = BTreeMap::new();
        for left in ["off", "on"] {
            for right in ["off", "on"] {
                dense.insert(
                    output(left, right),
                    compiled
                        .joint_probability(&input("walking"), &output(left, right))
                        .unwrap(),
                );
            }
        }

        assert_eq!(factorized.len(), dense.len());
        for entry in factorized {
            let assignment = BTreeMap::from([
                (
                    "left_leg_state".to_owned(),
                    entry.values["left_leg_state"].clone(),
                ),
                (
                    "right_leg_state".to_owned(),
                    entry.values["right_leg_state"].clone(),
                ),
            ]);
            assert_eq!(Some(&entry.probability), dense.get(&assignment));
        }
        assert_eq!(
            dense
                .values()
                .copied()
                .try_fold(Rational::zero(), |sum, value| { sum.checked_add(value) })
                .unwrap(),
            Rational::one()
        );
    }

    #[test]
    fn sparse_coupling_is_not_equivalent_to_independent_legs() {
        let compiled = spec().compile().unwrap();
        let left = compiled
            .marginal(&input("walking"), &["left_leg_state".to_owned()], 8)
            .unwrap();
        let right = compiled
            .marginal(&input("walking"), &["right_leg_state".to_owned()], 8)
            .unwrap();
        let joint = compiled
            .joint_probability(&input("walking"), &output("on", "on"))
            .unwrap();
        let left_on = left
            .iter()
            .find(|entry| entry.values["left_leg_state"] == "on")
            .unwrap()
            .probability;
        let right_on = right
            .iter()
            .find(|entry| entry.values["right_leg_state"] == "on")
            .unwrap()
            .probability;
        let independent = left_on.checked_mul(right_on).unwrap();
        assert_eq!(joint, rational(27, 40));
        assert_ne!(joint, independent);
    }

    #[test]
    fn query_subset_eliminates_unrequested_leg_exactly() {
        let compiled = spec().compile().unwrap();
        let marginal = compiled
            .marginal(&input("walking"), &["right_leg_state".to_owned()], 4)
            .unwrap();
        let active = marginal
            .iter()
            .find(|entry| entry.values["right_leg_state"] == "on")
            .unwrap()
            .probability;
        assert_eq!(active, rational(7, 10));
    }

    #[test]
    fn semantic_storage_order_does_not_change_compiled_model() {
        let original = spec().compile().unwrap();
        let mut reordered = spec();
        reordered.variables.reverse();
        reordered.factors.reverse();
        for factor in &mut reordered.factors {
            factor.parent_variable_ids.reverse();
            for row in &mut factor.rows {
                row.parent_values.reverse();
                row.outcomes.reverse();
            }
            factor.rows.reverse();
        }
        reordered.source_manifests.reverse();
        let compiled_reordered = reordered.compile().unwrap();
        assert_eq!(original, compiled_reordered);
        assert_eq!(
            original
                .marginal(
                    &input("walking"),
                    &["left_leg_state".to_owned(), "right_leg_state".to_owned()],
                    16,
                )
                .unwrap(),
            compiled_reordered
                .marginal(
                    &input("walking"),
                    &["right_leg_state".to_owned(), "left_leg_state".to_owned()],
                    16,
                )
                .unwrap()
        );
    }

    #[test]
    fn local_action_scope_declares_direct_target_and_coupled_descendant() {
        let compiled = scoped_action_spec().compile().unwrap();
        assert_eq!(compiled.intervention_scopes.len(), 1);
        let scope = &compiled.intervention_scopes[0];
        assert_eq!(compiled.variables[scope.action_variable].id, "action");
        assert_eq!(
            scope
                .direct_targets
                .iter()
                .map(|index| compiled.variables[*index].id.as_str())
                .collect::<Vec<_>>(),
            vec!["left_leg_state"]
        );
        assert_eq!(
            scope
                .allowed_descendants
                .iter()
                .map(|index| compiled.variables[*index].id.as_str())
                .collect::<Vec<_>>(),
            vec!["left_leg_state", "right_leg_state"]
        );

        let inputs = BTreeMap::from([("action".to_owned(), "activate_left".to_owned())]);
        let outputs = output("on", "on");
        assert_eq!(
            compiled.joint_probability(&inputs, &outputs).unwrap(),
            rational(9, 10)
        );
    }

    #[test]
    fn unannounced_direct_action_effect_on_non_target_leg_is_rejected() {
        let mut invalid = scoped_action_spec();
        let right = invalid
            .factors
            .iter_mut()
            .find(|factor| factor.target_variable_id == "right_leg_state")
            .unwrap();
        right.parent_variable_ids = vec!["action".to_owned(), "left_leg_state".to_owned()];
        right.rows = vec![
            conditional_row(&["activate_left", "off"], vec![outcome("off", 1, 1)]),
            conditional_row(
                &["activate_left", "on"],
                vec![outcome("off", 1, 10), outcome("on", 9, 10)],
            ),
            conditional_row(&["idle", "off"], vec![outcome("off", 1, 1)]),
            conditional_row(
                &["idle", "on"],
                vec![outcome("off", 1, 10), outcome("on", 9, 10)],
            ),
        ];
        assert!(matches!(invalid.compile(), Err(FactorError::Invalid(_))));
    }

    #[test]
    fn action_variable_without_scope_contract_is_rejected() {
        let mut invalid = scoped_action_spec();
        invalid.intervention_scopes.clear();
        assert!(matches!(invalid.compile(), Err(FactorError::Invalid(_))));
    }

    #[test]
    fn action_scope_cannot_omit_a_declared_coupling_descendant() {
        let mut invalid = scoped_action_spec();
        invalid.intervention_scopes[0].allowed_descendant_output_ids =
            vec!["left_leg_state".to_owned()];
        assert!(matches!(invalid.compile(), Err(FactorError::Invalid(_))));
    }

    #[test]
    fn thirty_binary_local_variables_support_single_leg_marginal_without_dense_expansion() {
        let variables = std::iter::once(DiscreteVariable {
            id: "context".to_owned(),
            role: VariableRole::Input,
            domain: vec!["walking".to_owned()],
        })
        .chain((0..30).map(|index| DiscreteVariable {
            id: format!("leg_{index:02}"),
            role: VariableRole::Output,
            domain: vec!["off".to_owned(), "on".to_owned()],
        }))
        .collect();
        let factors = (0..30)
            .map(|index| {
                factor(
                    &format!("leg-transition-{index:02}"),
                    &format!("leg_{index:02}"),
                    &[],
                    vec![conditional_row(
                        &[],
                        vec![outcome("off", 1, 2), outcome("on", 1, 2)],
                    )],
                )
            })
            .collect();
        let compiled = FactorizedTransitionSpec {
            source_manifests: vec![fixture_source()],
            variables,
            factors,
            intervention_scopes: vec![],
        }
        .compile()
        .unwrap();
        assert_eq!(compiled.dense_output_cardinality().unwrap(), 1_073_741_824);

        let marginal = compiled
            .marginal(&input("walking"), &["leg_17".to_owned()], 2)
            .unwrap();
        assert_eq!(marginal.len(), 2);
        assert!(
            marginal
                .iter()
                .all(|entry| entry.probability == rational(1, 2))
        );
    }

    #[test]
    fn non_normalized_conditional_row_fails_closed() {
        let mut invalid = spec();
        invalid.factors[0].rows[0].outcomes = vec![outcome("off", 1, 3), outcome("on", 1, 3)];
        assert!(matches!(invalid.compile(), Err(FactorError::Invalid(_))));
    }

    #[test]
    fn cyclic_output_dependencies_are_rejected() {
        let mut invalid = spec();
        let left = invalid
            .factors
            .iter_mut()
            .find(|factor| factor.target_variable_id == "left_leg_state")
            .unwrap();
        left.parent_variable_ids = vec!["right_leg_state".to_owned()];
        left.rows = vec![
            conditional_row(&["off"], vec![outcome("off", 1, 2), outcome("on", 1, 2)]),
            conditional_row(&["on"], vec![outcome("off", 1, 2), outcome("on", 1, 2)]),
        ];
        assert!(matches!(invalid.compile(), Err(FactorError::CycleDetected)));
    }

    #[test]
    fn each_factor_requires_source_provenance() {
        let mut invalid = spec();
        invalid.factors[0].source_manifest_ids.clear();
        assert!(matches!(
            invalid.compile(),
            Err(FactorError::MissingSource(_))
        ));
    }

    #[test]
    fn dense_output_materialization_is_rejected_before_allocation() {
        let variables = std::iter::once(DiscreteVariable {
            id: "action".to_owned(),
            role: VariableRole::Input,
            domain: vec!["idle".to_owned()],
        })
        .chain((0..16).map(|index| DiscreteVariable {
            id: format!("leg_{index:02}"),
            role: VariableRole::Output,
            domain: vec!["off".to_owned(), "on".to_owned()],
        }))
        .collect();
        let factors = (0..16)
            .map(|index| {
                factor(
                    &format!("transition_{index:02}"),
                    &format!("leg_{index:02}"),
                    &[],
                    vec![conditional_row(
                        &[],
                        vec![outcome("off", 1, 2), outcome("on", 1, 2)],
                    )],
                )
            })
            .collect();
        let large = FactorizedTransitionSpec {
            source_manifests: vec![fixture_source()],
            variables,
            factors,
            intervention_scopes: vec![],
        }
        .compile()
        .unwrap();
        assert_eq!(large.dense_output_cardinality().unwrap(), 65_536);
        assert!(matches!(
            large.enumerate_dense_outputs(
                &BTreeMap::from([("action".to_owned(), "idle".to_owned())]),
                1_024
            ),
            Err(FactorError::ResourceLimit {
                requested: 65_536,
                limit: 1_024
            })
        ));
    }
}
