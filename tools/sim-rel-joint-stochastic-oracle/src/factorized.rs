//! Exact sparse/factorized state-transition model for multi-leg locomotion.
//!
//! The dense `JointKernel` remains the reference for enumerable models. This
//! module represents a transition as one conditional probability distribution
//! (CPD) per next-state variable and action, then answers marginals by exact
//! variable elimination. A dense joint distribution is materialized only when
//! its full Cartesian size is within an explicit caller-supplied budget.

use crate::Rational;
use crate::locomotion::{
    ClaimCeiling, EvidenceClass, SourceDigestPin, SourceManifest, source_supports_claim,
    valid_sha256, validate_source,
};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

const DEFAULT_MAX_PARENT_ASSIGNMENTS: usize = 65_536;
const DEFAULT_MAX_FACTOR_CELLS: usize = 65_536;
const DEFAULT_MAX_INTERMEDIATE_CELLS: usize = 262_144;
const DEFAULT_MAX_DENSE_MATERIALIZATION_CELLS: usize = 65_536;
const DEFAULT_MAX_TOTAL_CPDS: usize = 1_000_000;

/// Whether a CPD parent reads the current or next value of a state variable.
/// Next-state parents are permitted only when the resulting dependency graph
/// is acyclic; this permits explicit local-to-intersegmental coupling.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ParentScope {
    Current,
    Next,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ParentRef {
    pub scope: ParentScope,
    pub variable_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateVariable {
    pub id: String,
    /// Semantic value IDs, not display labels. Compilation sorts these IDs.
    pub domain: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionDefinition {
    pub id: String,
    /// Variables intentionally targeted by this intervention.
    pub target_variable_ids: Vec<String>,
    /// Variables allowed to change only when reachable through declared
    /// next-state dependencies from a target. This makes coupling explicit.
    pub allowed_descendant_variable_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalOutcome {
    pub value_id: String,
    pub probability: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalRow {
    /// Exactly one value for every ParentRef, keyed by semantic variable ID.
    pub parent_values: BTreeMap<String, String>,
    /// Exactly one probability for every value in the target variable domain.
    pub outcomes: Vec<ConditionalOutcome>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionalFactor {
    pub action_id: String,
    pub target_variable_id: String,
    pub parents: Vec<ParentRef>,
    pub rows: Vec<ConditionalRow>,
    pub source_manifest_ids: Vec<String>,
    pub claim_ceiling: ClaimCeiling,
}

/// A fully declarative factored transition model. All resource ceilings are
/// explicit and checked before allocating a Cartesian factor or dense joint.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorizedTransitionSpec {
    pub variables: Vec<StateVariable>,
    pub actions: Vec<ActionDefinition>,
    pub source_manifests: Vec<SourceManifest>,
    pub source_pins: Vec<SourceDigestPin>,
    pub factors: Vec<ConditionalFactor>,
    pub max_parent_assignments: usize,
    pub max_factor_cells: usize,
    pub max_intermediate_cells: usize,
    pub max_dense_materialization_cells: usize,
    pub max_total_cpds: usize,
}

impl FactorizedTransitionSpec {
    pub fn new(
        variables: Vec<StateVariable>,
        actions: Vec<ActionDefinition>,
        source_manifests: Vec<SourceManifest>,
        source_pins: Vec<SourceDigestPin>,
        factors: Vec<ConditionalFactor>,
    ) -> Self {
        Self {
            variables,
            actions,
            source_manifests,
            source_pins,
            factors,
            max_parent_assignments: DEFAULT_MAX_PARENT_ASSIGNMENTS,
            max_factor_cells: DEFAULT_MAX_FACTOR_CELLS,
            max_intermediate_cells: DEFAULT_MAX_INTERMEDIATE_CELLS,
            max_dense_materialization_cells: DEFAULT_MAX_DENSE_MATERIALIZATION_CELLS,
            max_total_cpds: DEFAULT_MAX_TOTAL_CPDS,
        }
    }

    /// Canonicalize by semantic IDs and validate every variable, CPD, source,
    /// action effect boundary and declared next-state dependency.
    pub fn compile(&self) -> Result<CompiledFactorizedTransition, FactorizedError> {
        if self.variables.is_empty() || self.actions.is_empty() || self.factors.is_empty() {
            return Err(FactorizedError::InvalidSpec(
                "variables, actions and factors must be non-empty".to_owned(),
            ));
        }
        if self.max_parent_assignments == 0
            || self.max_factor_cells == 0
            || self.max_intermediate_cells == 0
            || self.max_dense_materialization_cells == 0
            || self.max_total_cpds == 0
        {
            return Err(FactorizedError::InvalidSpec(
                "all resource ceilings must be positive".to_owned(),
            ));
        }
        if self.factors.len() > self.max_total_cpds {
            return Err(FactorizedError::ResourceLimitExceeded {
                stage: "conditional factors",
                requested: self.factors.len(),
                limit: self.max_total_cpds,
            });
        }

        let mut variables = self.variables.clone();
        variables.sort_by(|left, right| left.id.cmp(&right.id));
        for variable in &mut variables {
            require_nonempty(&variable.id, "state variable ID")?;
            if variable.domain.is_empty() {
                return Err(FactorizedError::InvalidSpec(format!(
                    "state variable '{}' has an empty domain",
                    variable.id
                )));
            }
            variable.domain.sort();
            if has_duplicates(&variable.domain) {
                return Err(FactorizedError::InvalidSpec(format!(
                    "state variable '{}' has duplicate domain values",
                    variable.id
                )));
            }
            for value in &variable.domain {
                require_nonempty(value, "domain value")?;
            }
        }
        if has_duplicates_by(&variables, |item| item.id.as_str()) {
            return Err(FactorizedError::InvalidSpec(
                "duplicate state variable IDs".to_owned(),
            ));
        }
        let domains = variables
            .iter()
            .map(|variable| (variable.id.clone(), variable.domain.clone()))
            .collect::<BTreeMap<_, _>>();

        let mut actions = self.actions.clone();
        actions.sort_by(|left, right| left.id.cmp(&right.id));
        for action in &mut actions {
            require_nonempty(&action.id, "action ID")?;
            action.target_variable_ids.sort();
            action.allowed_descendant_variable_ids.sort();
            if action.target_variable_ids.is_empty() {
                return Err(FactorizedError::InvalidSpec(format!(
                    "action '{}' has no declared target variables",
                    action.id
                )));
            }
            if has_duplicates(&action.target_variable_ids)
                || has_duplicates(&action.allowed_descendant_variable_ids)
            {
                return Err(FactorizedError::InvalidSpec(format!(
                    "action '{}' has duplicate target/descendant variables",
                    action.id
                )));
            }
            for variable_id in action
                .target_variable_ids
                .iter()
                .chain(action.allowed_descendant_variable_ids.iter())
            {
                if !domains.contains_key(variable_id) {
                    return Err(FactorizedError::UnknownVariable(variable_id.clone()));
                }
            }
            if action.target_variable_ids.iter().any(|id| {
                action
                    .allowed_descendant_variable_ids
                    .binary_search(id)
                    .is_ok()
            }) {
                return Err(FactorizedError::InvalidSpec(format!(
                    "action '{}' declares a target as its own descendant",
                    action.id
                )));
            }
        }
        if has_duplicates_by(&actions, |item| item.id.as_str()) {
            return Err(FactorizedError::InvalidSpec(
                "duplicate action IDs".to_owned(),
            ));
        }
        let action_ids = actions
            .iter()
            .map(|action| action.id.clone())
            .collect::<BTreeSet<_>>();

        let mut sources = self.source_manifests.clone();
        sources.sort_by(|left, right| left.id.cmp(&right.id));
        for source in &sources {
            validate_source(source)
                .map_err(|_| FactorizedError::InvalidSource(source.id.clone()))?;
        }
        if has_duplicates_by(&sources, |item| item.id.as_str()) {
            return Err(FactorizedError::InvalidSpec(
                "duplicate source manifest IDs".to_owned(),
            ));
        }
        let source_map = sources
            .iter()
            .map(|source| (source.id.as_str(), source))
            .collect::<BTreeMap<_, _>>();
        let mut pins = self.source_pins.clone();
        pins.sort_by(|left, right| left.source_manifest_id.cmp(&right.source_manifest_id));
        let mut pinned = BTreeSet::new();
        for pin in &pins {
            let source = source_map
                .get(pin.source_manifest_id.as_str())
                .ok_or_else(|| FactorizedError::UnknownSource(pin.source_manifest_id.clone()))?;
            if !valid_sha256(&pin.expected_sha256) {
                return Err(FactorizedError::InvalidSource(
                    pin.source_manifest_id.clone(),
                ));
            }
            if !pinned.insert(pin.source_manifest_id.clone()) {
                return Err(FactorizedError::InvalidSpec(format!(
                    "duplicate source digest pin '{}'",
                    pin.source_manifest_id
                )));
            }
            if source.sha256.as_deref() != Some(pin.expected_sha256.as_str()) {
                return Err(FactorizedError::SourcePinMismatch(
                    pin.source_manifest_id.clone(),
                ));
            }
        }
        for source in &sources {
            if source.evidence_class != EvidenceClass::SyntheticFixture
                && !pinned.contains(&source.id)
            {
                return Err(FactorizedError::UnpinnedSource(source.id.clone()));
            }
        }

        let mut factors = self.factors.clone();
        for factor in &mut factors {
            require_nonempty(&factor.action_id, "factor action ID")?;
            require_nonempty(&factor.target_variable_id, "factor target variable ID")?;
            if !action_ids.contains(&factor.action_id) {
                return Err(FactorizedError::UnknownAction(factor.action_id.clone()));
            }
            if !domains.contains_key(&factor.target_variable_id) {
                return Err(FactorizedError::UnknownVariable(
                    factor.target_variable_id.clone(),
                ));
            }
            factor.parents.sort();
            let parent_ids = factor
                .parents
                .iter()
                .map(|parent| parent.variable_id.clone())
                .collect::<Vec<_>>();
            let mut seen_parent_ids = BTreeSet::new();
            if parent_ids
                .iter()
                .any(|parent_id| !seen_parent_ids.insert(parent_id.clone()))
            {
                return Err(FactorizedError::InvalidSpec(format!(
                    "factor '{}:{}' has duplicate parent variable IDs",
                    factor.action_id, factor.target_variable_id
                )));
            }
            if factor.parents.iter().any(|parent| {
                parent.scope == ParentScope::Next && parent.variable_id == factor.target_variable_id
            }) {
                return Err(FactorizedError::InvalidSpec(format!(
                    "factor '{}:{}' cannot depend on its own next-state value",
                    factor.action_id, factor.target_variable_id
                )));
            }
            for parent in &factor.parents {
                if !domains.contains_key(&parent.variable_id) {
                    return Err(FactorizedError::UnknownVariable(parent.variable_id.clone()));
                }
            }
            if factor.source_manifest_ids.is_empty() {
                return Err(FactorizedError::MissingFactorSource {
                    action_id: factor.action_id.clone(),
                    target_variable_id: factor.target_variable_id.clone(),
                });
            }
            factor.source_manifest_ids.sort();
            if has_duplicates(&factor.source_manifest_ids) {
                return Err(FactorizedError::InvalidSpec(format!(
                    "factor '{}:{}' repeats a source manifest ID",
                    factor.action_id, factor.target_variable_id
                )));
            }
            let supports_claim =
                factor
                    .source_manifest_ids
                    .iter()
                    .try_fold(false, |supported, source_id| {
                        let source = source_map
                            .get(source_id.as_str())
                            .ok_or_else(|| FactorizedError::UnknownSource(source_id.clone()))?;
                        Ok::<bool, FactorizedError>(
                            supported || source_supports_claim(source, factor.claim_ceiling),
                        )
                    })?;
            if !supports_claim {
                return Err(FactorizedError::InvalidClaim {
                    action_id: factor.action_id.clone(),
                    target_variable_id: factor.target_variable_id.clone(),
                });
            }

            let parent_assignment_count =
                factor.parents.iter().try_fold(1_usize, |product, parent| {
                    let cardinality = domains[&parent.variable_id].len();
                    product.checked_mul(cardinality).ok_or_else(|| {
                        FactorizedError::Arithmetic("parent assignment count overflow".to_owned())
                    })
                })?;
            if parent_assignment_count > self.max_parent_assignments {
                return Err(FactorizedError::ResourceLimitExceeded {
                    stage: "CPD parent assignments",
                    requested: parent_assignment_count,
                    limit: self.max_parent_assignments,
                });
            }
            let target_cardinality = domains[&factor.target_variable_id].len();
            let cpd_cells = parent_assignment_count
                .checked_mul(target_cardinality)
                .ok_or_else(|| FactorizedError::Arithmetic("CPD cell count overflow".to_owned()))?;
            if cpd_cells > self.max_factor_cells {
                return Err(FactorizedError::ResourceLimitExceeded {
                    stage: "CPD cells",
                    requested: cpd_cells,
                    limit: self.max_factor_cells,
                });
            }

            factor.rows.sort_by(|left, right| {
                row_key_from_map(&factor.parents, &left.parent_values)
                    .cmp(&row_key_from_map(&factor.parents, &right.parent_values))
            });
            if factor.rows.len() != parent_assignment_count {
                return Err(FactorizedError::IncompleteCpd {
                    action_id: factor.action_id.clone(),
                    target_variable_id: factor.target_variable_id.clone(),
                });
            }
            let mut row_keys = BTreeSet::new();
            for row in &mut factor.rows {
                if row.parent_values.len() != factor.parents.len()
                    || factor
                        .parents
                        .iter()
                        .any(|parent| !row.parent_values.contains_key(&parent.variable_id))
                {
                    return Err(FactorizedError::InvalidParentAssignment {
                        action_id: factor.action_id.clone(),
                        target_variable_id: factor.target_variable_id.clone(),
                    });
                }
                for parent in &factor.parents {
                    let value = row.parent_values.get(&parent.variable_id).ok_or_else(|| {
                        FactorizedError::InvalidParentAssignment {
                            action_id: factor.action_id.clone(),
                            target_variable_id: factor.target_variable_id.clone(),
                        }
                    })?;
                    if domains[&parent.variable_id].binary_search(value).is_err() {
                        return Err(FactorizedError::InvalidParentValue {
                            variable_id: parent.variable_id.clone(),
                            value_id: value.clone(),
                        });
                    }
                }
                let key = row_key_from_map(&factor.parents, &row.parent_values);
                if !row_keys.insert(key) {
                    return Err(FactorizedError::DuplicateParentAssignment {
                        action_id: factor.action_id.clone(),
                        target_variable_id: factor.target_variable_id.clone(),
                    });
                }
                row.outcomes
                    .sort_by(|left, right| left.value_id.cmp(&right.value_id));
                let target_domain = &domains[&factor.target_variable_id];
                if row.outcomes.len() != target_domain.len()
                    || row
                        .outcomes
                        .iter()
                        .map(|outcome| outcome.value_id.as_str())
                        .collect::<Vec<_>>()
                        != target_domain.iter().map(String::as_str).collect::<Vec<_>>()
                {
                    return Err(FactorizedError::IncompleteOutcomes {
                        action_id: factor.action_id.clone(),
                        target_variable_id: factor.target_variable_id.clone(),
                    });
                }
                let mut mass = Rational::zero();
                for outcome in &row.outcomes {
                    if !outcome.probability.is_probability() {
                        return Err(FactorizedError::InvalidProbability {
                            action_id: factor.action_id.clone(),
                            target_variable_id: factor.target_variable_id.clone(),
                        });
                    }
                    mass = mass
                        .checked_add(outcome.probability)
                        .map_err(FactorizedError::Arithmetic)?;
                }
                if mass != Rational::one() {
                    return Err(FactorizedError::UnnormalizedCpd {
                        action_id: factor.action_id.clone(),
                        target_variable_id: factor.target_variable_id.clone(),
                    });
                }
            }

            // Verify that the supplied table covers the entire Cartesian
            // parent scope, not merely the expected number of distinct rows.
            let expected_keys = enumerate_parent_keys(&factor.parents, &domains)?;
            if row_keys != expected_keys {
                return Err(FactorizedError::IncompleteCpd {
                    action_id: factor.action_id.clone(),
                    target_variable_id: factor.target_variable_id.clone(),
                });
            }
        }
        factors.sort_by(|left, right| {
            (&left.action_id, &left.target_variable_id)
                .cmp(&(&right.action_id, &right.target_variable_id))
        });

        let mut factor_keys = BTreeSet::new();
        for factor in &factors {
            if !factor_keys.insert((factor.action_id.clone(), factor.target_variable_id.clone())) {
                return Err(FactorizedError::DuplicateFactor {
                    action_id: factor.action_id.clone(),
                    target_variable_id: factor.target_variable_id.clone(),
                });
            }
        }
        for action in &actions {
            for variable in &variables {
                if !factor_keys.contains(&(action.id.clone(), variable.id.clone())) {
                    return Err(FactorizedError::MissingFactor {
                        action_id: action.id.clone(),
                        target_variable_id: variable.id.clone(),
                    });
                }
            }
        }

        let mut compiled_factors = Vec::with_capacity(factors.len());
        let mut dependency_orders = BTreeMap::new();
        for action in &actions {
            let action_factors = factors
                .iter()
                .filter(|factor| factor.action_id == action.id)
                .collect::<Vec<_>>();
            let order = topological_order(&variables, &action_factors)?;
            validate_action_targets_have_possible_effect(action, &action_factors, &domains)?;
            validate_action_effect_boundary(action, &action_factors, &variables)?;
            dependency_orders.insert(action.id.clone(), order);
        }
        for factor in &factors {
            let mut rows = BTreeMap::new();
            for row in &factor.rows {
                let key = row_key_from_map(&factor.parents, &row.parent_values);
                let probabilities = row
                    .outcomes
                    .iter()
                    .map(|outcome| outcome.probability)
                    .collect::<Vec<_>>();
                rows.insert(key, probabilities);
            }
            compiled_factors.push(CompiledConditionalFactor {
                action_id: factor.action_id.clone(),
                target_variable_id: factor.target_variable_id.clone(),
                parents: factor.parents.clone(),
                rows,
                source_manifest_ids: factor.source_manifest_ids.clone(),
                claim_ceiling: factor.claim_ceiling,
            });
        }

        Ok(CompiledFactorizedTransition {
            variables,
            actions,
            factors: compiled_factors,
            dependency_orders,
            source_manifests: sources,
            source_pins: pins,
            max_factor_cells: self.max_factor_cells,
            max_intermediate_cells: self.max_intermediate_cells,
            max_dense_materialization_cells: self.max_dense_materialization_cells,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CompiledConditionalFactor {
    action_id: String,
    target_variable_id: String,
    parents: Vec<ParentRef>,
    /// Key order is exactly the canonical ParentRef order; probability order
    /// is exactly the canonical target-domain order.
    rows: BTreeMap<Vec<String>, Vec<Rational>>,
    source_manifest_ids: Vec<String>,
    claim_ceiling: ClaimCeiling,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledFactorizedTransition {
    variables: Vec<StateVariable>,
    actions: Vec<ActionDefinition>,
    factors: Vec<CompiledConditionalFactor>,
    dependency_orders: BTreeMap<String, Vec<String>>,
    source_manifests: Vec<SourceManifest>,
    source_pins: Vec<SourceDigestPin>,
    max_factor_cells: usize,
    max_intermediate_cells: usize,
    max_dense_materialization_cells: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarginalOutcome {
    pub values: BTreeMap<String, String>,
    pub probability: Rational,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InferenceStats {
    /// Deterministic variable elimination order chosen for this query.
    pub elimination_order: Vec<String>,
    /// Sum of initial instantiated CPD factor-table cells.
    pub instantiated_factor_cells: usize,
    /// Largest table allocated by instantiation, multiplication or elimination.
    pub peak_intermediate_cells: usize,
    /// Largest variable scope among instantiated and elimination factors,
    /// excluding the final requested output scope.
    pub peak_intermediate_scope_size: usize,
    /// Output-cell × (input-factor-count - 1) multiplication estimate.
    pub factor_product_multiplications: usize,
    /// Number of exact additions executed by sum-out operations.
    pub sum_out_additions: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorEvidenceBinding {
    pub target_variable_id: String,
    pub source_manifest_ids: Vec<String>,
    pub claim_ceiling: ClaimCeiling,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorMarginal {
    pub action_id: String,
    /// Query variable IDs in canonical order.
    pub variable_ids: Vec<String>,
    pub outcomes: Vec<MarginalOutcome>,
    /// Conservative evidence trace: one binding for every CPD in this action's
    /// transition model. The compiled model retains the referenced manifests
    /// and digest pins so this can be independently resolved.
    pub factor_evidence: Vec<FactorEvidenceBinding>,
    pub stats: InferenceStats,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeightedNextState {
    pub values: BTreeMap<String, String>,
    pub probability: Rational,
}

impl CompiledFactorizedTransition {
    pub fn variable_ids(&self) -> Vec<String> {
        self.variables
            .iter()
            .map(|variable| variable.id.clone())
            .collect()
    }

    pub fn action_ids(&self) -> Vec<String> {
        self.actions
            .iter()
            .map(|action| action.id.clone())
            .collect()
    }

    pub fn source_manifests(&self) -> &[SourceManifest] {
        &self.source_manifests
    }

    pub fn source_pins(&self) -> &[SourceDigestPin] {
        &self.source_pins
    }

    pub fn dependency_order(&self, action_id: &str) -> Result<&[String], FactorizedError> {
        self.dependency_orders
            .get(action_id)
            .map(Vec::as_slice)
            .ok_or_else(|| FactorizedError::UnknownAction(action_id.to_owned()))
    }

    /// Exact sum-product / variable elimination for a marginal over query
    /// variables. No full joint Cartesian state is allocated unless the
    /// elimination order itself requires an intermediate factor of that size.
    pub fn marginal_next(
        &self,
        current_state: &BTreeMap<String, String>,
        action_id: &str,
        query_variable_ids: &[String],
    ) -> Result<FactorMarginal, FactorizedError> {
        self.validate_assignment(current_state, "current")?;
        if !self.actions.iter().any(|action| action.id == action_id) {
            return Err(FactorizedError::UnknownAction(action_id.to_owned()));
        }
        let mut query = query_variable_ids.to_vec();
        query.sort();
        if has_duplicates(&query) {
            return Err(FactorizedError::InvalidSpec(
                "marginal query repeats a variable ID".to_owned(),
            ));
        }
        for variable_id in &query {
            if !self
                .variables
                .iter()
                .any(|variable| &variable.id == variable_id)
            {
                return Err(FactorizedError::UnknownVariable(variable_id.clone()));
            }
        }

        let domains = self.domain_map();
        let action_factors = self
            .factors
            .iter()
            .filter(|factor| factor.action_id == action_id)
            .collect::<Vec<_>>();
        let factor_evidence = action_factors
            .iter()
            .map(|factor| FactorEvidenceBinding {
                target_variable_id: factor.target_variable_id.clone(),
                source_manifest_ids: factor.source_manifest_ids.clone(),
                claim_ceiling: factor.claim_ceiling,
            })
            .collect::<Vec<_>>();
        let mut stats = InferenceStats::default();
        let mut tables = Vec::new();
        for factor in &action_factors {
            let table = self.instantiate_factor(factor, current_state, &domains)?;
            stats.instantiated_factor_cells = stats
                .instantiated_factor_cells
                .checked_add(table.values.len())
                .ok_or_else(|| {
                    FactorizedError::Arithmetic(
                        "instantiated factor-cell count overflow".to_owned(),
                    )
                })?;
            record_table_stats(&mut stats, &table);
            tables.push(table);
        }

        let mut remaining = self
            .variables
            .iter()
            .map(|variable| variable.id.clone())
            .filter(|id| query.binary_search(id).is_err())
            .collect::<BTreeSet<_>>();
        while !remaining.is_empty() {
            let variable_id = choose_elimination_variable(&tables, &remaining, &domains)?;
            let (bucket, rest) = partition_tables(&tables, &variable_id);
            if bucket.is_empty() {
                return Err(FactorizedError::InvalidSpec(format!(
                    "variable '{variable_id}' has no factor during elimination"
                )));
            }
            let product = multiply_tables(&bucket, &domains, self.max_intermediate_cells)?;
            record_table_stats(&mut stats, &product);
            stats.factor_product_multiplications = stats
                .factor_product_multiplications
                .checked_add(
                    product
                        .values
                        .len()
                        .checked_mul(bucket.len().saturating_sub(1))
                        .ok_or_else(|| {
                            FactorizedError::Arithmetic(
                                "factor multiplication work count overflow".to_owned(),
                            )
                        })?,
                )
                .ok_or_else(|| {
                    FactorizedError::Arithmetic("factor multiplication count overflow".to_owned())
                })?;
            let reduced = sum_out_variable(
                &product,
                &variable_id,
                &domains,
                self.max_intermediate_cells,
            )?;
            record_table_stats(&mut stats, &reduced);
            stats.sum_out_additions = stats
                .sum_out_additions
                .checked_add(
                    reduced
                        .values
                        .len()
                        .saturating_mul(domains[&variable_id].len()),
                )
                .ok_or_else(|| {
                    FactorizedError::Arithmetic("sum-out operation count overflow".to_owned())
                })?;
            stats.elimination_order.push(variable_id.clone());
            tables = rest;
            tables.push(reduced);
            remaining.remove(&variable_id);
        }

        let result = multiply_tables(&tables, &domains, self.max_intermediate_cells)?;
        // The requested output scope is not an elimination intermediate.
        // Its cell count still contributes to peak allocated memory, while
        // peak_intermediate_scope_size reports the factor width of the
        // actual elimination process rather than the user's query width.
        stats.peak_intermediate_cells = stats.peak_intermediate_cells.max(result.values.len());
        stats.factor_product_multiplications = stats
            .factor_product_multiplications
            .checked_add(
                result
                    .values
                    .len()
                    .checked_mul(tables.len().saturating_sub(1))
                    .ok_or_else(|| {
                        FactorizedError::Arithmetic(
                            "final factor multiplication work count overflow".to_owned(),
                        )
                    })?,
            )
            .ok_or_else(|| {
                FactorizedError::Arithmetic("final factor multiplication count overflow".to_owned())
            })?;
        if result.scope != query {
            return Err(FactorizedError::InvalidSpec(
                "variable elimination did not preserve exactly the requested query scope"
                    .to_owned(),
            ));
        }
        let total = result
            .values
            .iter()
            .try_fold(Rational::zero(), |sum, value| {
                sum.checked_add(*value).map_err(FactorizedError::Arithmetic)
            })?;
        if total != Rational::one() {
            return Err(FactorizedError::UnnormalizedMarginal);
        }

        let mut outcomes = Vec::new();
        for (index, probability) in result.values.iter().enumerate() {
            if probability.is_zero() {
                continue;
            }
            let decoded = decode_assignment(index, &result.scope, &domains)?;
            outcomes.push(MarginalOutcome {
                values: result.scope.iter().cloned().zip(decoded).collect(),
                probability: *probability,
            });
        }
        Ok(FactorMarginal {
            action_id: action_id.to_owned(),
            variable_ids: query,
            outcomes,
            factor_evidence,
            stats,
        })
    }

    /// Materialize the full next-state distribution only when its Cartesian
    /// cardinality is within max_dense_materialization_cells.
    pub fn materialize_joint(
        &self,
        current_state: &BTreeMap<String, String>,
        action_id: &str,
    ) -> Result<Vec<WeightedNextState>, FactorizedError> {
        let cell_count = self
            .variables
            .iter()
            .try_fold(1_usize, |product, variable| {
                product.checked_mul(variable.domain.len()).ok_or_else(|| {
                    FactorizedError::Arithmetic("dense joint cardinality overflow".to_owned())
                })
            })?;
        if cell_count > self.max_dense_materialization_cells {
            return Err(FactorizedError::ResourceLimitExceeded {
                stage: "dense joint materialization",
                requested: cell_count,
                limit: self.max_dense_materialization_cells,
            });
        }
        let query = self.variable_ids();
        let marginal = self.marginal_next(current_state, action_id, &query)?;
        Ok(marginal
            .outcomes
            .into_iter()
            .map(|outcome| WeightedNextState {
                values: outcome.values,
                probability: outcome.probability,
            })
            .collect())
    }

    /// Independent direct product evaluation for one specified transition.
    /// This is useful as a correctness cross-check for variable elimination.
    pub fn transition_probability(
        &self,
        current_state: &BTreeMap<String, String>,
        action_id: &str,
        next_state: &BTreeMap<String, String>,
    ) -> Result<Rational, FactorizedError> {
        self.validate_assignment(current_state, "current")?;
        self.validate_assignment(next_state, "next")?;
        if !self.actions.iter().any(|action| action.id == action_id) {
            return Err(FactorizedError::UnknownAction(action_id.to_owned()));
        }
        let mut joint = Rational::one();
        for factor in self
            .factors
            .iter()
            .filter(|factor| factor.action_id == action_id)
        {
            let key = factor
                .parents
                .iter()
                .map(|parent| {
                    match parent.scope {
                        ParentScope::Current => current_state.get(&parent.variable_id),
                        ParentScope::Next => next_state.get(&parent.variable_id),
                    }
                    .cloned()
                    .ok_or_else(|| {
                        FactorizedError::InvalidAssignment(format!(
                            "missing parent '{}' for factor '{}'",
                            parent.variable_id, factor.target_variable_id
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let probabilities = factor.rows.get(&key).ok_or_else(|| {
                FactorizedError::InvalidSpec(format!(
                    "compiled CPD row missing for factor '{}'",
                    factor.target_variable_id
                ))
            })?;
            let target_value = next_state.get(&factor.target_variable_id).ok_or_else(|| {
                FactorizedError::InvalidAssignment(format!(
                    "next state is missing '{}'",
                    factor.target_variable_id
                ))
            })?;
            let target_domain = &self
                .variables
                .iter()
                .find(|variable| variable.id == factor.target_variable_id)
                .expect("compiled factors reference known variables")
                .domain;
            let outcome_index = target_domain.binary_search(target_value).map_err(|_| {
                FactorizedError::InvalidAssignment(format!(
                    "invalid next value '{}' for '{}'",
                    target_value, factor.target_variable_id
                ))
            })?;
            joint = joint
                .checked_mul(probabilities[outcome_index])
                .map_err(FactorizedError::Arithmetic)?;
        }
        Ok(joint)
    }

    fn validate_assignment(
        &self,
        assignment: &BTreeMap<String, String>,
        label: &'static str,
    ) -> Result<(), FactorizedError> {
        if assignment.len() != self.variables.len() {
            return Err(FactorizedError::InvalidAssignment(format!(
                "{label} assignment must provide exactly {} variables",
                self.variables.len()
            )));
        }
        for variable in &self.variables {
            let value = assignment.get(&variable.id).ok_or_else(|| {
                FactorizedError::InvalidAssignment(format!(
                    "{label} assignment is missing '{}'",
                    variable.id
                ))
            })?;
            if variable.domain.binary_search(value).is_err() {
                return Err(FactorizedError::InvalidAssignment(format!(
                    "{label} assignment has unknown value '{}' for '{}'",
                    value, variable.id
                )));
            }
        }
        Ok(())
    }

    fn domain_map(&self) -> BTreeMap<String, Vec<String>> {
        self.variables
            .iter()
            .map(|variable| (variable.id.clone(), variable.domain.clone()))
            .collect()
    }

    fn instantiate_factor(
        &self,
        factor: &CompiledConditionalFactor,
        current_state: &BTreeMap<String, String>,
        domains: &BTreeMap<String, Vec<String>>,
    ) -> Result<FactorTable, FactorizedError> {
        let mut scope = factor
            .parents
            .iter()
            .filter(|parent| parent.scope == ParentScope::Next)
            .map(|parent| parent.variable_id.clone())
            .collect::<Vec<_>>();
        scope.push(factor.target_variable_id.clone());
        scope.sort();
        scope.dedup();

        let cells = scope.iter().try_fold(1_usize, |product, id| {
            product
                .checked_mul(domains[id].len())
                .ok_or_else(|| FactorizedError::Arithmetic("factor cell count overflow".to_owned()))
        })?;
        if cells > self.max_factor_cells {
            return Err(FactorizedError::ResourceLimitExceeded {
                stage: "instantiated factor cells",
                requested: cells,
                limit: self.max_factor_cells,
            });
        }

        let mut values = Vec::with_capacity(cells);
        for index in 0..cells {
            let decoded = decode_assignment(index, &scope, domains)?;
            let assignment = scope
                .iter()
                .cloned()
                .zip(decoded)
                .collect::<BTreeMap<_, _>>();
            let parent_key = factor
                .parents
                .iter()
                .map(|parent| {
                    let value = match parent.scope {
                        ParentScope::Current => current_state.get(&parent.variable_id),
                        ParentScope::Next => assignment.get(&parent.variable_id),
                    };
                    value.cloned().ok_or_else(|| {
                        FactorizedError::InvalidAssignment(format!(
                            "factor '{}' is missing parent '{}'",
                            factor.target_variable_id, parent.variable_id
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let probabilities = factor.rows.get(&parent_key).ok_or_else(|| {
                FactorizedError::InvalidSpec(format!(
                    "factor '{}' has no CPD row for a declared parent assignment",
                    factor.target_variable_id
                ))
            })?;
            let target_value = assignment.get(&factor.target_variable_id).ok_or_else(|| {
                FactorizedError::InvalidAssignment(format!(
                    "factor '{}' does not include its target in the instantiated scope",
                    factor.target_variable_id
                ))
            })?;
            let target_domain = &domains[&factor.target_variable_id];
            let target_index = target_domain.binary_search(target_value).map_err(|_| {
                FactorizedError::InvalidAssignment(format!(
                    "unknown target value '{}' for '{}'",
                    target_value, factor.target_variable_id
                ))
            })?;
            values.push(probabilities[target_index]);
        }
        Ok(FactorTable { scope, values })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FactorTable {
    /// Canonically sorted variable IDs.
    scope: Vec<String>,
    /// Row-major mixed-radix order following scope and each variable's sorted domain.
    values: Vec<Rational>,
}

fn record_table_stats(stats: &mut InferenceStats, table: &FactorTable) {
    stats.peak_intermediate_cells = stats.peak_intermediate_cells.max(table.values.len());
    stats.peak_intermediate_scope_size = stats.peak_intermediate_scope_size.max(table.scope.len());
}

fn choose_elimination_variable(
    tables: &[FactorTable],
    remaining: &BTreeSet<String>,
    domains: &BTreeMap<String, Vec<String>>,
) -> Result<String, FactorizedError> {
    let mut best: Option<(usize, String)> = None;
    for variable_id in remaining {
        let mut union = BTreeSet::new();
        for table in tables
            .iter()
            .filter(|table| table.scope.binary_search(variable_id).is_ok())
        {
            union.extend(table.scope.iter().cloned());
        }
        let cells = union.iter().try_fold(1_usize, |product, id| {
            product
                .checked_mul(domains[id].len())
                .ok_or_else(|| FactorizedError::Arithmetic("elimination cost overflow".to_owned()))
        })?;
        let candidate = (cells, variable_id.clone());
        if best.as_ref().is_none_or(|current| candidate < *current) {
            best = Some(candidate);
        }
    }
    best.map(|(_, variable)| variable)
        .ok_or_else(|| FactorizedError::InvalidSpec("no eliminable variable remains".to_owned()))
}

fn partition_tables(
    tables: &[FactorTable],
    variable_id: &str,
) -> (Vec<FactorTable>, Vec<FactorTable>) {
    let mut bucket = Vec::new();
    let mut rest = Vec::new();
    for table in tables {
        if table
            .scope
            .binary_search_by(|id| id.as_str().cmp(variable_id))
            .is_ok()
        {
            bucket.push(table.clone());
        } else {
            rest.push(table.clone());
        }
    }
    (bucket, rest)
}

fn multiply_tables(
    tables: &[FactorTable],
    domains: &BTreeMap<String, Vec<String>>,
    max_cells: usize,
) -> Result<FactorTable, FactorizedError> {
    if tables.is_empty() {
        return Ok(FactorTable {
            scope: Vec::new(),
            values: vec![Rational::one()],
        });
    }
    let mut scope = tables
        .iter()
        .flat_map(|table| table.scope.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    scope.sort();
    let cells = scope.iter().try_fold(1_usize, |product, id| {
        product.checked_mul(domains[id].len()).ok_or_else(|| {
            FactorizedError::Arithmetic("intermediate factor size overflow".to_owned())
        })
    })?;
    if cells > max_cells {
        return Err(FactorizedError::ResourceLimitExceeded {
            stage: "variable-elimination intermediate factor",
            requested: cells,
            limit: max_cells,
        });
    }
    let mut values = Vec::with_capacity(cells);
    for index in 0..cells {
        let decoded = decode_assignment(index, &scope, domains)?;
        let assignment = scope
            .iter()
            .cloned()
            .zip(decoded)
            .collect::<BTreeMap<_, _>>();
        let mut product = Rational::one();
        for table in tables {
            let table_index = table_index_for_assignment(table, &assignment, domains)?;
            product = product
                .checked_mul(table.values[table_index])
                .map_err(FactorizedError::Arithmetic)?;
        }
        values.push(product);
    }
    Ok(FactorTable { scope, values })
}

fn sum_out_variable(
    table: &FactorTable,
    variable_id: &str,
    domains: &BTreeMap<String, Vec<String>>,
    max_cells: usize,
) -> Result<FactorTable, FactorizedError> {
    let position = table
        .scope
        .binary_search_by(|id| id.as_str().cmp(variable_id))
        .map_err(|_| {
            FactorizedError::InvalidSpec(format!(
                "cannot eliminate absent variable '{variable_id}'"
            ))
        })?;
    let mut scope = table.scope.clone();
    scope.remove(position);
    let cells = scope.iter().try_fold(1_usize, |product, id| {
        product
            .checked_mul(domains[id].len())
            .ok_or_else(|| FactorizedError::Arithmetic("summed factor size overflow".to_owned()))
    })?;
    if cells > max_cells {
        return Err(FactorizedError::ResourceLimitExceeded {
            stage: "variable-elimination summed factor",
            requested: cells,
            limit: max_cells,
        });
    }
    let mut values = Vec::with_capacity(cells);
    for index in 0..cells {
        let decoded = decode_assignment(index, &scope, domains)?;
        let mut assignment = scope
            .iter()
            .cloned()
            .zip(decoded)
            .collect::<BTreeMap<_, _>>();
        let mut total = Rational::zero();
        for variable_value in &domains[variable_id] {
            assignment.insert(variable_id.to_owned(), variable_value.clone());
            let table_index = table_index_for_assignment(table, &assignment, domains)?;
            total = total
                .checked_add(table.values[table_index])
                .map_err(FactorizedError::Arithmetic)?;
        }
        values.push(total);
    }
    Ok(FactorTable { scope, values })
}

fn table_index_for_assignment(
    table: &FactorTable,
    assignment: &BTreeMap<String, String>,
    domains: &BTreeMap<String, Vec<String>>,
) -> Result<usize, FactorizedError> {
    let mut index = 0_usize;
    for variable_id in &table.scope {
        let value = assignment.get(variable_id).ok_or_else(|| {
            FactorizedError::InvalidAssignment(format!(
                "factor lookup is missing variable '{variable_id}'"
            ))
        })?;
        let value_index = domains[variable_id].binary_search(value).map_err(|_| {
            FactorizedError::InvalidAssignment(format!(
                "factor lookup has unknown value '{value}' for '{variable_id}'"
            ))
        })?;
        index = index
            .checked_mul(domains[variable_id].len())
            .and_then(|value| value.checked_add(value_index))
            .ok_or_else(|| FactorizedError::Arithmetic("factor table index overflow".to_owned()))?;
    }
    Ok(index)
}

fn decode_assignment(
    mut index: usize,
    scope: &[String],
    domains: &BTreeMap<String, Vec<String>>,
) -> Result<Vec<String>, FactorizedError> {
    let mut values = vec![String::new(); scope.len()];
    for position in (0..scope.len()).rev() {
        let domain = &domains[&scope[position]];
        let cardinality = domain.len();
        if cardinality == 0 {
            return Err(FactorizedError::InvalidSpec(format!(
                "variable '{}' has an empty domain",
                scope[position]
            )));
        }
        let value_index = index % cardinality;
        index /= cardinality;
        values[position] = domain[value_index].clone();
    }
    if index != 0 {
        return Err(FactorizedError::InvalidSpec(
            "factor table index exceeds its declared scope".to_owned(),
        ));
    }
    Ok(values)
}

fn enumerate_parent_keys(
    parents: &[ParentRef],
    domains: &BTreeMap<String, Vec<String>>,
) -> Result<BTreeSet<Vec<String>>, FactorizedError> {
    let cells = parents.iter().try_fold(1_usize, |product, parent| {
        product
            .checked_mul(domains[&parent.variable_id].len())
            .ok_or_else(|| FactorizedError::Arithmetic("parent key count overflow".to_owned()))
    })?;
    let mut keys = BTreeSet::new();
    for index in 0..cells {
        let key = decode_assignment(
            index,
            &parents
                .iter()
                .map(|parent| parent.variable_id.clone())
                .collect::<Vec<_>>(),
            domains,
        )?;
        keys.insert(key);
    }
    Ok(keys)
}

fn row_key_from_map(parents: &[ParentRef], values: &BTreeMap<String, String>) -> Vec<String> {
    parents
        .iter()
        .map(|parent| values.get(&parent.variable_id).cloned().unwrap_or_default())
        .collect()
}

fn topological_order(
    variables: &[StateVariable],
    factors: &[&ConditionalFactor],
) -> Result<Vec<String>, FactorizedError> {
    let mut indegree = variables
        .iter()
        .map(|variable| (variable.id.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut children = variables
        .iter()
        .map(|variable| (variable.id.clone(), BTreeSet::<String>::new()))
        .collect::<BTreeMap<_, _>>();
    for factor in factors {
        for parent in factor
            .parents
            .iter()
            .filter(|parent| parent.scope == ParentScope::Next)
        {
            if children
                .get_mut(&parent.variable_id)
                .expect("parents were validated")
                .insert(factor.target_variable_id.clone())
            {
                *indegree
                    .get_mut(&factor.target_variable_id)
                    .expect("targets were validated") += 1;
            }
        }
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(variables.len());
    while let Some(next) = ready.iter().next().cloned() {
        ready.remove(&next);
        order.push(next.clone());
        for child in &children[&next] {
            let child_degree = indegree.get_mut(child).expect("child is known");
            *child_degree -= 1;
            if *child_degree == 0 {
                ready.insert(child.clone());
            }
        }
    }
    if order.len() != variables.len() {
        return Err(FactorizedError::CyclicNextStateDependencies);
    }
    Ok(order)
}

fn validate_action_targets_have_possible_effect(
    action: &ActionDefinition,
    factors: &[&ConditionalFactor],
    domains: &BTreeMap<String, Vec<String>>,
) -> Result<(), FactorizedError> {
    for target_variable_id in &action.target_variable_ids {
        let factor = factors
            .iter()
            .find(|factor| factor.target_variable_id == *target_variable_id)
            .ok_or_else(|| FactorizedError::MissingFactor {
                action_id: action.id.clone(),
                target_variable_id: target_variable_id.clone(),
            })?;
        let domain = &domains[target_variable_id];
        let current_self_parent = factor.parents.iter().any(|parent| {
            parent.scope == ParentScope::Current && parent.variable_id == *target_variable_id
        });
        let has_possible_change = if current_self_parent {
            factor.rows.iter().any(|row| {
                let current_value = row.parent_values.get(target_variable_id);
                current_value.is_some_and(|current_value| {
                    row.outcomes.iter().any(|outcome| {
                        outcome.value_id != *current_value && !outcome.probability.is_zero()
                    })
                })
            })
        } else {
            domain.len() > 1
        };
        if !has_possible_change {
            return Err(FactorizedError::ActionTargetHasNoPossibleEffect {
                action_id: action.id.clone(),
                variable_id: target_variable_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_action_effect_boundary(
    action: &ActionDefinition,
    factors: &[&ConditionalFactor],
    variables: &[StateVariable],
) -> Result<(), FactorizedError> {
    let target_set = action
        .target_variable_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let descendant_set = action
        .allowed_descendant_variable_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let affected_set = target_set
        .union(&descendant_set)
        .cloned()
        .collect::<BTreeSet<_>>();

    // Every declared descendant must be reachable from an intervention target
    // through explicit Next-scope parent edges in this action's graph.
    let mut children = BTreeMap::<String, BTreeSet<String>>::new();
    for factor in factors {
        for parent in factor
            .parents
            .iter()
            .filter(|parent| parent.scope == ParentScope::Next)
        {
            children
                .entry(parent.variable_id.clone())
                .or_default()
                .insert(factor.target_variable_id.clone());
        }
    }
    for descendant in &action.allowed_descendant_variable_ids {
        let mut frontier = action.target_variable_ids.to_vec();
        let mut visited = BTreeSet::new();
        let mut reachable = false;
        while let Some(variable) = frontier.pop() {
            if !visited.insert(variable.clone()) {
                continue;
            }
            if &variable == descendant {
                reachable = true;
                break;
            }
            if let Some(next) = children.get(&variable) {
                frontier.extend(next.iter().cloned());
            }
        }
        if !reachable {
            return Err(FactorizedError::UndeclaredOrUnreachableEffect {
                action_id: action.id.clone(),
                variable_id: descendant.clone(),
            });
        }
    }

    // Any variable outside the declared target/descendant closure must be an
    // exact identity transition for every possible CPD parent assignment.
    for factor in factors {
        if affected_set.contains(&factor.target_variable_id) {
            continue;
        }
        let self_parent = factor.parents.iter().find(|parent| {
            parent.scope == ParentScope::Current && parent.variable_id == factor.target_variable_id
        });
        let self_parent =
            self_parent.ok_or_else(|| FactorizedError::UndeclaredOrUnreachableEffect {
                action_id: action.id.clone(),
                variable_id: factor.target_variable_id.clone(),
            })?;
        let _ = self_parent;
        for row in &factor.rows {
            let current_value = row
                .parent_values
                .get(&factor.target_variable_id)
                .expect("identity current parent has a row value");
            let matching = row
                .outcomes
                .iter()
                .find(|outcome| outcome.value_id == *current_value)
                .expect("outcomes cover the target domain");
            if matching.probability != Rational::one()
                || row.outcomes.iter().any(|outcome| {
                    outcome.value_id != *current_value && !outcome.probability.is_zero()
                })
            {
                return Err(FactorizedError::UndeclaredOrUnreachableEffect {
                    action_id: action.id.clone(),
                    variable_id: factor.target_variable_id.clone(),
                });
            }
        }
    }
    let _ = variables;
    Ok(())
}

fn require_nonempty(value: &str, label: &'static str) -> Result<(), FactorizedError> {
    if value.trim().is_empty() {
        Err(FactorizedError::InvalidSpec(format!(
            "{label} must be non-empty"
        )))
    } else {
        Ok(())
    }
}

fn has_duplicates(values: &[String]) -> bool {
    values.windows(2).any(|pair| pair[0] == pair[1])
}

fn has_duplicates_by<T, F>(values: &[T], key: F) -> bool
where
    F: Fn(&T) -> &str,
{
    let mut seen = BTreeSet::new();
    values.iter().any(|value| !seen.insert(key(value)))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FactorizedError {
    InvalidSpec(String),
    InvalidSource(String),
    UnknownSource(String),
    UnpinnedSource(String),
    SourcePinMismatch(String),
    InvalidClaim {
        action_id: String,
        target_variable_id: String,
    },
    MissingFactorSource {
        action_id: String,
        target_variable_id: String,
    },
    UnknownAction(String),
    UnknownVariable(String),
    DuplicateFactor {
        action_id: String,
        target_variable_id: String,
    },
    MissingFactor {
        action_id: String,
        target_variable_id: String,
    },
    IncompleteCpd {
        action_id: String,
        target_variable_id: String,
    },
    InvalidParentAssignment {
        action_id: String,
        target_variable_id: String,
    },
    InvalidParentValue {
        variable_id: String,
        value_id: String,
    },
    DuplicateParentAssignment {
        action_id: String,
        target_variable_id: String,
    },
    IncompleteOutcomes {
        action_id: String,
        target_variable_id: String,
    },
    InvalidProbability {
        action_id: String,
        target_variable_id: String,
    },
    UnnormalizedCpd {
        action_id: String,
        target_variable_id: String,
    },
    CyclicNextStateDependencies,
    UndeclaredOrUnreachableEffect {
        action_id: String,
        variable_id: String,
    },
    ActionTargetHasNoPossibleEffect {
        action_id: String,
        variable_id: String,
    },
    InvalidAssignment(String),
    UnnormalizedMarginal,
    ResourceLimitExceeded {
        stage: &'static str,
        requested: usize,
        limit: usize,
    },
    Arithmetic(String),
}

impl Display for FactorizedError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for FactorizedError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_source() -> SourceManifest {
        SourceManifest {
            id: "synthetic-factor-fixture".to_owned(),
            kind: crate::locomotion::SourceKind::SyntheticFixture,
            evidence_class: EvidenceClass::SyntheticFixture,
            citation_or_source_uri: "synthetic://factorized-locomotion".to_owned(),
            dataset_id: None,
            release_or_materialization: None,
            sha256: Some("a".repeat(64)),
            unknown_endpoint_inventory_complete: true,
            unknown_endpoints: Vec::new(),
        }
    }

    fn parent(scope: ParentScope, variable_id: &str) -> ParentRef {
        ParentRef {
            scope,
            variable_id: variable_id.to_owned(),
        }
    }

    fn row(parents: &[(&str, &str)], outcomes: &[(&str, u128, u128)]) -> ConditionalRow {
        ConditionalRow {
            parent_values: parents
                .iter()
                .map(|(id, value)| ((*id).to_owned(), (*value).to_owned()))
                .collect(),
            outcomes: outcomes
                .iter()
                .map(|(id, numerator, denominator)| ConditionalOutcome {
                    value_id: (*id).to_owned(),
                    probability: Rational::new(*numerator, *denominator).unwrap(),
                })
                .collect(),
        }
    }

    fn fixture() -> FactorizedTransitionSpec {
        let variables = vec![
            StateVariable {
                id: "context".to_owned(),
                domain: vec!["walking".to_owned(), "other".to_owned()],
            },
            StateVariable {
                id: "left-contact".to_owned(),
                domain: vec!["free".to_owned(), "loaded".to_owned()],
            },
            StateVariable {
                id: "right-contact".to_owned(),
                domain: vec!["free".to_owned(), "loaded".to_owned()],
            },
        ];
        let actions = vec![ActionDefinition {
            id: "perturb-left".to_owned(),
            target_variable_ids: vec!["left-contact".to_owned()],
            allowed_descendant_variable_ids: vec!["right-contact".to_owned()],
        }];
        let source_manifests = vec![synthetic_source()];
        let factors = vec![
            ConditionalFactor {
                action_id: "perturb-left".to_owned(),
                target_variable_id: "context".to_owned(),
                parents: vec![parent(ParentScope::Current, "context")],
                rows: vec![
                    row(
                        &[("context", "other")],
                        &[("other", 1, 1), ("walking", 0, 1)],
                    ),
                    row(
                        &[("context", "walking")],
                        &[("other", 0, 1), ("walking", 1, 1)],
                    ),
                ],
                source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
                claim_ceiling: ClaimCeiling::SyntheticQualification,
            },
            ConditionalFactor {
                action_id: "perturb-left".to_owned(),
                target_variable_id: "left-contact".to_owned(),
                parents: vec![parent(ParentScope::Current, "context")],
                rows: vec![
                    row(&[("context", "other")], &[("free", 3, 4), ("loaded", 1, 4)]),
                    row(
                        &[("context", "walking")],
                        &[("free", 1, 4), ("loaded", 3, 4)],
                    ),
                ],
                source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
                claim_ceiling: ClaimCeiling::SyntheticQualification,
            },
            ConditionalFactor {
                action_id: "perturb-left".to_owned(),
                target_variable_id: "right-contact".to_owned(),
                parents: vec![parent(ParentScope::Next, "left-contact")],
                rows: vec![
                    row(
                        &[("left-contact", "free")],
                        &[("free", 9, 10), ("loaded", 1, 10)],
                    ),
                    row(
                        &[("left-contact", "loaded")],
                        &[("free", 1, 10), ("loaded", 9, 10)],
                    ),
                ],
                source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
                claim_ceiling: ClaimCeiling::SyntheticQualification,
            },
        ];
        FactorizedTransitionSpec::new(variables, actions, source_manifests, Vec::new(), factors)
    }

    fn identity_factor(variable_id: &str, domain: &[&str]) -> ConditionalFactor {
        let rows = domain
            .iter()
            .map(|current_value| ConditionalRow {
                parent_values: BTreeMap::from([(
                    variable_id.to_owned(),
                    (*current_value).to_owned(),
                )]),
                outcomes: domain
                    .iter()
                    .map(|next_value| ConditionalOutcome {
                        value_id: (*next_value).to_owned(),
                        probability: if next_value == current_value {
                            Rational::one()
                        } else {
                            Rational::zero()
                        },
                    })
                    .collect(),
            })
            .collect();
        ConditionalFactor {
            action_id: "walk-front-left".to_owned(),
            target_variable_id: variable_id.to_owned(),
            parents: vec![parent(ParentScope::Current, variable_id)],
            rows,
            source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
            claim_ceiling: ClaimCeiling::SyntheticQualification,
        }
    }

    fn pathway_factor(
        action_id: &str,
        target_variable_id: &str,
        parents: Vec<ParentRef>,
        rows: Vec<ConditionalRow>,
    ) -> ConditionalFactor {
        ConditionalFactor {
            action_id: action_id.to_owned(),
            target_variable_id: target_variable_id.to_owned(),
            parents,
            rows,
            source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
            claim_ceiling: ClaimCeiling::SyntheticQualification,
        }
    }

    fn identity_factor_for(
        action_id: &str,
        variable_id: &str,
        domain: &[&str],
    ) -> ConditionalFactor {
        let rows = domain
            .iter()
            .map(|current_value| ConditionalRow {
                parent_values: BTreeMap::from([(
                    variable_id.to_owned(),
                    (*current_value).to_owned(),
                )]),
                outcomes: domain
                    .iter()
                    .map(|next_value| ConditionalOutcome {
                        value_id: (*next_value).to_owned(),
                        probability: if next_value == current_value {
                            Rational::one()
                        } else {
                            Rational::zero()
                        },
                    })
                    .collect(),
            })
            .collect();
        pathway_factor(
            action_id,
            variable_id,
            vec![parent(ParentScope::Current, variable_id)],
            rows,
        )
    }

    fn parallel_descending_gate_fixture() -> FactorizedTransitionSpec {
        let variables = vec![
            StateVariable {
                id: "behavior-context".to_owned(),
                domain: vec!["grooming".to_owned(), "walking".to_owned()],
            },
            StateVariable {
                id: "descending-excitatory".to_owned(),
                domain: vec!["high".to_owned(), "low".to_owned()],
            },
            StateVariable {
                id: "descending-inhibitory".to_owned(),
                domain: vec!["high".to_owned(), "low".to_owned()],
            },
            StateVariable {
                id: "local-9a-activity".to_owned(),
                domain: vec!["active".to_owned(), "inactive".to_owned()],
            },
            StateVariable {
                id: "hook-movement-gain".to_owned(),
                domain: vec!["available".to_owned(), "suppressed".to_owned()],
            },
            StateVariable {
                id: "claw-position-gain".to_owned(),
                domain: vec!["available".to_owned()],
            },
        ];
        let actions = vec![
            ActionDefinition {
                id: "drive-excitatory-pathway".to_owned(),
                target_variable_ids: vec!["descending-excitatory".to_owned()],
                allowed_descendant_variable_ids: vec![
                    "hook-movement-gain".to_owned(),
                    "local-9a-activity".to_owned(),
                ],
            },
            ActionDefinition {
                id: "drive-inhibitory-pathway".to_owned(),
                target_variable_ids: vec!["descending-inhibitory".to_owned()],
                allowed_descendant_variable_ids: vec![
                    "hook-movement-gain".to_owned(),
                    "local-9a-activity".to_owned(),
                ],
            },
        ];
        let source_manifests = vec![synthetic_source()];
        let mut factors = Vec::new();

        for action_id in ["drive-excitatory-pathway", "drive-inhibitory-pathway"] {
            factors.push(identity_factor_for(
                action_id,
                "behavior-context",
                &["grooming", "walking"],
            ));
            factors.push(identity_factor_for(
                action_id,
                "claw-position-gain",
                &["available"],
            ));

            let is_excitatory_action = action_id == "drive-excitatory-pathway";
            let targeted = if is_excitatory_action {
                "descending-excitatory"
            } else {
                "descending-inhibitory"
            };
            let non_target = if is_excitatory_action {
                "descending-inhibitory"
            } else {
                "descending-excitatory"
            };
            factors.push(pathway_factor(
                action_id,
                targeted,
                Vec::new(),
                vec![row(&[], &[("high", 1, 1), ("low", 0, 1)])],
            ));
            factors.push(identity_factor_for(action_id, non_target, &["high", "low"]));

            // This is a deliberately synthetic interaction truth table:
            // strong excitatory drive with low inhibitory drive activates
            // the local 9A-like gate; all other combinations leave it inactive.
            // It encodes no claim about the measured sign or gain of either
            // descending pathway in the Dallmann preparation.
            factors.push(pathway_factor(
                action_id,
                "local-9a-activity",
                vec![
                    parent(ParentScope::Next, "descending-excitatory"),
                    parent(ParentScope::Next, "descending-inhibitory"),
                ],
                vec![
                    row(
                        &[
                            ("descending-excitatory", "high"),
                            ("descending-inhibitory", "high"),
                        ],
                        &[("active", 0, 1), ("inactive", 1, 1)],
                    ),
                    row(
                        &[
                            ("descending-excitatory", "high"),
                            ("descending-inhibitory", "low"),
                        ],
                        &[("active", 1, 1), ("inactive", 0, 1)],
                    ),
                    row(
                        &[
                            ("descending-excitatory", "low"),
                            ("descending-inhibitory", "high"),
                        ],
                        &[("active", 0, 1), ("inactive", 1, 1)],
                    ),
                    row(
                        &[
                            ("descending-excitatory", "low"),
                            ("descending-inhibitory", "low"),
                        ],
                        &[("active", 0, 1), ("inactive", 1, 1)],
                    ),
                ],
            ));
            factors.push(pathway_factor(
                action_id,
                "hook-movement-gain",
                vec![parent(ParentScope::Next, "local-9a-activity")],
                vec![
                    row(
                        &[("local-9a-activity", "active")],
                        &[("available", 0, 1), ("suppressed", 1, 1)],
                    ),
                    row(
                        &[("local-9a-activity", "inactive")],
                        &[("available", 1, 1), ("suppressed", 0, 1)],
                    ),
                ],
            ));
        }

        FactorizedTransitionSpec::new(variables, actions, source_manifests, Vec::new(), factors)
    }

    fn six_leg_sparse_fixture() -> FactorizedTransitionSpec {
        let leg_names = [
            "front-left",
            "front-right",
            "middle-left",
            "middle-right",
            "hind-left",
            "hind-right",
        ];
        let mut variables = vec![
            StateVariable {
                id: "context".to_owned(),
                domain: vec!["other".to_owned(), "walking".to_owned()],
            },
            StateVariable {
                id: "acquisition".to_owned(),
                domain: vec!["calibrated".to_owned(), "offset".to_owned()],
            },
        ];

        for leg in leg_names {
            variables.extend([
                StateVariable {
                    id: format!("{leg}-phase"),
                    domain: vec!["stance".to_owned(), "swing".to_owned()],
                },
                StateVariable {
                    id: format!("{leg}-contact"),
                    domain: vec!["free".to_owned(), "loaded".to_owned()],
                },
                StateVariable {
                    id: format!("{leg}-force"),
                    domain: vec!["high".to_owned(), "low".to_owned()],
                },
            ]);
        }

        let mut actions = Vec::with_capacity(leg_names.len());
        let mut factors = Vec::with_capacity(leg_names.len() * variables.len());
        for target_leg in leg_names {
            let action_id = format!("walk-{target_leg}");
            let target_phase = format!("{target_leg}-phase");
            let target_contact = format!("{target_leg}-contact");
            let target_force = format!("{target_leg}-force");
            actions.push(ActionDefinition {
                id: action_id.clone(),
                target_variable_ids: vec![target_phase.clone()],
                allowed_descendant_variable_ids: vec![target_contact.clone(), target_force.clone()],
            });

            factors.push(identity_factor_for(
                &action_id,
                "context",
                &["other", "walking"],
            ));
            factors.push(identity_factor_for(
                &action_id,
                "acquisition",
                &["calibrated", "offset"],
            ));

            for leg in leg_names {
                let phase = format!("{leg}-phase");
                let contact = format!("{leg}-contact");
                let force = format!("{leg}-force");
                if leg != target_leg {
                    factors.push(identity_factor_for(
                        &action_id,
                        &phase,
                        &["stance", "swing"],
                    ));
                    factors.push(identity_factor_for(
                        &action_id,
                        &contact,
                        &["free", "loaded"],
                    ));
                    factors.push(identity_factor_for(&action_id, &force, &["high", "low"]));
                    continue;
                }

                factors.push(ConditionalFactor {
                    action_id: action_id.clone(),
                    target_variable_id: phase.clone(),
                    parents: vec![parent(ParentScope::Current, "context")],
                    rows: vec![
                        row(
                            &[("context", "other")],
                            &[("stance", 3, 4), ("swing", 1, 4)],
                        ),
                        row(
                            &[("context", "walking")],
                            &[("stance", 1, 4), ("swing", 3, 4)],
                        ),
                    ],
                    source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
                    claim_ceiling: ClaimCeiling::SyntheticQualification,
                });
                factors.push(ConditionalFactor {
                    action_id: action_id.clone(),
                    target_variable_id: contact.clone(),
                    parents: vec![parent(ParentScope::Next, &phase)],
                    rows: vec![
                        row(&[(&phase, "stance")], &[("free", 1, 1), ("loaded", 0, 1)]),
                        row(&[(&phase, "swing")], &[("free", 0, 1), ("loaded", 1, 1)]),
                    ],
                    source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
                    claim_ceiling: ClaimCeiling::SyntheticQualification,
                });
                factors.push(ConditionalFactor {
                    action_id: action_id.clone(),
                    target_variable_id: force.clone(),
                    parents: vec![parent(ParentScope::Next, &contact)],
                    rows: vec![
                        row(&[(&contact, "free")], &[("high", 1, 20), ("low", 19, 20)]),
                        row(&[(&contact, "loaded")], &[("high", 9, 10), ("low", 1, 10)]),
                    ],
                    source_manifest_ids: vec!["synthetic-factor-fixture".to_owned()],
                    claim_ceiling: ClaimCeiling::SyntheticQualification,
                });
            }
        }

        FactorizedTransitionSpec::new(
            variables,
            actions,
            vec![synthetic_source()],
            Vec::new(),
            factors,
        )
    }

    fn walking_current() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("context".to_owned(), "walking".to_owned()),
            ("left-contact".to_owned(), "free".to_owned()),
            ("right-contact".to_owned(), "free".to_owned()),
        ])
    }

    #[test]
    fn variable_elimination_matches_independent_direct_joint_product() {
        let compiled = fixture().compile().unwrap();
        let current = walking_current();
        let dense = compiled
            .materialize_joint(&current, "perturb-left")
            .unwrap();
        assert_eq!(dense.len(), 4);
        let mut direct = BTreeMap::new();
        for context in ["other", "walking"] {
            for left in ["free", "loaded"] {
                for right in ["free", "loaded"] {
                    let next = BTreeMap::from([
                        ("context".to_owned(), context.to_owned()),
                        ("left-contact".to_owned(), left.to_owned()),
                        ("right-contact".to_owned(), right.to_owned()),
                    ]);
                    let probability = compiled
                        .transition_probability(&current, "perturb-left", &next)
                        .unwrap();
                    if !probability.is_zero() {
                        direct.insert(next, probability);
                    }
                }
            }
        }
        let eliminated = dense
            .into_iter()
            .map(|state| (state.values, state.probability))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(eliminated, direct);
    }

    #[test]
    fn marginal_results_retain_action_factor_evidence_and_source_manifests() {
        let compiled = fixture().compile().unwrap();
        assert_eq!(compiled.source_manifests().len(), 1);
        assert_eq!(
            compiled.source_manifests()[0].id,
            "synthetic-factor-fixture"
        );
        assert!(compiled.source_pins().is_empty());

        let marginal = compiled
            .marginal_next(
                &walking_current(),
                "perturb-left",
                &["left-contact".to_owned()],
            )
            .unwrap();
        assert_eq!(marginal.action_id, "perturb-left");
        assert_eq!(marginal.factor_evidence.len(), 3);
        assert!(marginal.factor_evidence.iter().all(|binding| {
            binding.source_manifest_ids == vec!["synthetic-factor-fixture".to_owned()]
                && binding.claim_ceiling == ClaimCeiling::SyntheticQualification
        }));
        assert!(
            marginal
                .factor_evidence
                .iter()
                .any(|binding| binding.target_variable_id == "right-contact")
        );
    }

    #[test]
    fn exact_marginals_preserve_declared_next_state_coupling() {
        let compiled = fixture().compile().unwrap();
        let current = walking_current();
        let left = compiled
            .marginal_next(&current, "perturb-left", &["left-contact".to_owned()])
            .unwrap();
        let right = compiled
            .marginal_next(&current, "perturb-left", &["right-contact".to_owned()])
            .unwrap();
        let probability_of = |marginal: &FactorMarginal, value: &str| {
            marginal
                .outcomes
                .iter()
                .find(|outcome| {
                    outcome
                        .values
                        .values()
                        .next()
                        .is_some_and(|actual| actual == value)
                })
                .map(|outcome| outcome.probability)
                .unwrap()
        };
        assert_eq!(
            probability_of(&left, "loaded"),
            Rational::new(3, 4).unwrap()
        );
        assert_eq!(
            probability_of(&right, "loaded"),
            Rational::new(7, 10).unwrap()
        );
    }

    #[test]
    fn canonicalization_is_invariant_to_factor_variable_and_row_storage_order() {
        let input = fixture();
        let original = input.compile().unwrap();
        let mut reordered = input;
        reordered.variables.reverse();
        for variable in &mut reordered.variables {
            variable.domain.reverse();
        }
        reordered.factors.reverse();
        for factor in &mut reordered.factors {
            factor.rows.reverse();
            factor.parents.reverse();
            for row in &mut factor.rows {
                row.outcomes.reverse();
            }
        }
        let permuted = reordered.compile().unwrap();
        assert_eq!(original, permuted);
    }

    #[test]
    fn missing_parent_assignment_is_rejected() {
        let mut input = fixture();
        input.factors[1].rows.pop();
        assert!(matches!(
            input.compile(),
            Err(FactorizedError::IncompleteCpd { .. })
        ));
    }

    #[test]
    fn cyclic_next_state_dependencies_are_rejected() {
        let mut input = fixture();
        input.factors[1].parents = vec![parent(ParentScope::Next, "right-contact")];
        input.factors[2].parents = vec![parent(ParentScope::Next, "left-contact")];
        input.factors[1].rows = vec![
            row(
                &[("right-contact", "free")],
                &[("free", 1, 2), ("loaded", 1, 2)],
            ),
            row(
                &[("right-contact", "loaded")],
                &[("free", 1, 2), ("loaded", 1, 2)],
            ),
        ];
        input.factors[2].rows = vec![
            row(
                &[("left-contact", "free")],
                &[("free", 1, 2), ("loaded", 1, 2)],
            ),
            row(
                &[("left-contact", "loaded")],
                &[("free", 1, 2), ("loaded", 1, 2)],
            ),
        ];
        assert_eq!(
            input.compile(),
            Err(FactorizedError::CyclicNextStateDependencies)
        );
    }

    #[test]
    fn undeclared_effect_on_a_non_target_variable_is_rejected() {
        let mut input = fixture();
        input.actions[0].allowed_descendant_variable_ids.clear();
        assert!(matches!(
            input.compile(),
            Err(FactorizedError::UndeclaredOrUnreachableEffect {
                variable_id,
                ..
            }) if variable_id == "right-contact"
        ));
    }

    #[test]
    fn materialization_budget_is_checked_before_dense_enumeration() {
        let mut input = fixture();
        input.max_dense_materialization_cells = 4;
        let compiled = input.compile().unwrap();
        assert!(matches!(
            compiled.materialize_joint(&walking_current(), "perturb-left"),
            Err(FactorizedError::ResourceLimitExceeded {
                stage: "dense joint materialization",
                requested: 8,
                limit: 4
            })
        ));
    }

    #[test]
    fn non_target_variables_are_required_to_be_identity_transitions() {
        let mut input = fixture();
        input.factors[0].rows[0].outcomes = vec![
            ConditionalOutcome {
                value_id: "other".to_owned(),
                probability: Rational::new(1, 2).unwrap(),
            },
            ConditionalOutcome {
                value_id: "walking".to_owned(),
                probability: Rational::new(1, 2).unwrap(),
            },
        ];
        assert!(matches!(
            input.compile(),
            Err(FactorizedError::UndeclaredOrUnreachableEffect {
                variable_id,
                ..
            }) if variable_id == "context"
        ));
    }

    #[test]
    fn factor_claim_ceiling_cannot_exceed_its_source() {
        let mut input = fixture();
        input.factors[0].claim_ceiling = ClaimCeiling::PeerReviewedMeasuredNeuralReproduction;
        assert!(matches!(
            input.compile(),
            Err(FactorizedError::InvalidClaim { .. })
        ));
    }

    #[test]
    fn an_action_target_must_have_a_possible_non_identity_transition() {
        let mut input = fixture();
        let left_contact = input
            .factors
            .iter_mut()
            .find(|factor| factor.target_variable_id == "left-contact")
            .unwrap();
        left_contact.parents = vec![parent(ParentScope::Current, "left-contact")];
        left_contact.rows = vec![
            row(
                &[("left-contact", "free")],
                &[("free", 1, 1), ("loaded", 0, 1)],
            ),
            row(
                &[("left-contact", "loaded")],
                &[("free", 0, 1), ("loaded", 1, 1)],
            ),
        ];
        assert!(matches!(
            input.compile(),
            Err(FactorizedError::ActionTargetHasNoPossibleEffect {
                action_id,
                variable_id
            }) if action_id == "perturb-left" && variable_id == "left-contact"
        ));
    }

    #[test]
    fn local_independent_ablation_removes_the_declared_inter_leg_effect() {
        let coupled = fixture().compile().unwrap();
        let mut independent_spec = fixture();
        independent_spec.actions[0]
            .allowed_descendant_variable_ids
            .clear();
        let right = independent_spec
            .factors
            .iter_mut()
            .find(|factor| factor.target_variable_id == "right-contact")
            .unwrap();
        right.parents = vec![parent(ParentScope::Current, "right-contact")];
        right.rows = vec![
            row(
                &[("right-contact", "free")],
                &[("free", 1, 1), ("loaded", 0, 1)],
            ),
            row(
                &[("right-contact", "loaded")],
                &[("free", 0, 1), ("loaded", 1, 1)],
            ),
        ];
        let independent = independent_spec.compile().unwrap();
        let current = walking_current();
        let coupled_right = coupled
            .marginal_next(&current, "perturb-left", &["right-contact".to_owned()])
            .unwrap();
        let independent_right = independent
            .marginal_next(&current, "perturb-left", &["right-contact".to_owned()])
            .unwrap();
        let loaded_mass = |marginal: &FactorMarginal| {
            marginal
                .outcomes
                .iter()
                .find(|outcome| outcome.values["right-contact"] == "loaded")
                .map(|outcome| outcome.probability)
                .unwrap_or(Rational::zero())
        };
        assert_eq!(loaded_mass(&coupled_right), Rational::new(7, 10).unwrap());
        assert_eq!(loaded_mass(&independent_right), Rational::zero());
    }

    #[test]
    fn bounded_intermediate_factor_width_fails_closed() {
        let mut input = fixture();
        input.max_intermediate_cells = 2;
        let compiled = input.compile().unwrap();
        assert!(matches!(
            compiled.marginal_next(
                &walking_current(),
                "perturb-left",
                &["right-contact".to_owned()]
            ),
            Err(FactorizedError::ResourceLimitExceeded {
                stage: "variable-elimination intermediate factor",
                requested: 4,
                limit: 2
            })
        ));
    }

    #[test]
    fn parent_scope_cardinality_is_bounded_before_cpd_enumeration() {
        let mut input = fixture();
        input.max_parent_assignments = 1;
        assert!(matches!(
            input.compile(),
            Err(FactorizedError::ResourceLimitExceeded {
                stage: "CPD parent assignments",
                requested: 2,
                limit: 1
            })
        ));
    }

    #[test]
    fn a_variable_cannot_be_declared_as_both_current_and_next_parent() {
        let mut input = fixture();
        let right = input
            .factors
            .iter_mut()
            .find(|factor| factor.target_variable_id == "right-contact")
            .unwrap();
        right
            .parents
            .push(parent(ParentScope::Current, "left-contact"));
        assert!(matches!(
            input.compile(),
            Err(FactorizedError::InvalidSpec(message))
                if message.contains("duplicate parent variable IDs")
        ));
    }

    #[test]
    fn six_leg_sparse_model_answers_local_marginal_without_dense_cartesian_materialization() {
        let compiled = six_leg_sparse_fixture().compile().unwrap();
        let mut current = BTreeMap::from([
            ("context".to_owned(), "walking".to_owned()),
            ("acquisition".to_owned(), "calibrated".to_owned()),
        ]);
        for leg in [
            "front-left",
            "front-right",
            "middle-left",
            "middle-right",
            "hind-left",
            "hind-right",
        ] {
            current.insert(format!("{leg}-phase"), "stance".to_owned());
            current.insert(format!("{leg}-contact"), "free".to_owned());
            current.insert(format!("{leg}-force"), "low".to_owned());
        }

        let force = compiled
            .marginal_next(
                &current,
                "walk-front-left",
                &["front-left-force".to_owned()],
            )
            .unwrap();
        let high_probability = force
            .outcomes
            .iter()
            .find(|outcome| outcome.values["front-left-force"] == "high")
            .unwrap()
            .probability;
        assert_eq!(high_probability, Rational::new(11, 16).unwrap());
        assert_eq!(force.stats.elimination_order.len(), 19);
        assert!(force.stats.peak_intermediate_cells <= 4);
        assert!(force.stats.peak_intermediate_scope_size <= 2);
        assert!(force.stats.instantiated_factor_cells <= 4 * 20);

        assert!(matches!(
            compiled.materialize_joint(&current, "walk-front-left"),
            Err(FactorizedError::ResourceLimitExceeded {
                stage: "dense joint materialization",
                requested: 1_048_576,
                limit: 65_536
            })
        ));
    }

    fn parallel_pathway_current_state() -> BTreeMap<String, String> {
        BTreeMap::from([
            ("behavior-context".to_owned(), "walking".to_owned()),
            ("descending-excitatory".to_owned(), "low".to_owned()),
            ("descending-inhibitory".to_owned(), "low".to_owned()),
            ("local-9a-activity".to_owned(), "inactive".to_owned()),
            ("hook-movement-gain".to_owned(), "available".to_owned()),
            ("claw-position-gain".to_owned(), "available".to_owned()),
        ])
    }

    #[test]
    fn all_six_legs_have_symmetric_local_policies_without_cartesian_expansion() {
        let compiled = six_leg_sparse_fixture().compile().unwrap();
        let current = BTreeMap::from([
            ("context".to_owned(), "walking".to_owned()),
            ("acquisition".to_owned(), "calibrated".to_owned()),
            ("front-left-phase".to_owned(), "stance".to_owned()),
            ("front-left-contact".to_owned(), "free".to_owned()),
            ("front-left-force".to_owned(), "low".to_owned()),
            ("front-right-phase".to_owned(), "stance".to_owned()),
            ("front-right-contact".to_owned(), "free".to_owned()),
            ("front-right-force".to_owned(), "low".to_owned()),
            ("middle-left-phase".to_owned(), "stance".to_owned()),
            ("middle-left-contact".to_owned(), "free".to_owned()),
            ("middle-left-force".to_owned(), "low".to_owned()),
            ("middle-right-phase".to_owned(), "stance".to_owned()),
            ("middle-right-contact".to_owned(), "free".to_owned()),
            ("middle-right-force".to_owned(), "low".to_owned()),
            ("hind-left-phase".to_owned(), "stance".to_owned()),
            ("hind-left-contact".to_owned(), "free".to_owned()),
            ("hind-left-force".to_owned(), "low".to_owned()),
            ("hind-right-phase".to_owned(), "stance".to_owned()),
            ("hind-right-contact".to_owned(), "free".to_owned()),
            ("hind-right-force".to_owned(), "low".to_owned()),
        ]);
        let legs = [
            "front-left",
            "front-right",
            "middle-left",
            "middle-right",
            "hind-left",
            "hind-right",
        ];

        assert_eq!(compiled.action_ids().len(), 6);
        for leg in legs {
            let action = format!("walk-{leg}");
            let target_force = format!("{leg}-force");
            let other_contact = format!(
                "{}-contact",
                if leg == "front-left" {
                    "front-right"
                } else {
                    "front-left"
                }
            );
            let marginal = compiled
                .marginal_next(
                    &current,
                    &action,
                    &[target_force.clone(), other_contact.clone()],
                )
                .unwrap();

            let high_mass = marginal
                .outcomes
                .iter()
                .filter(|outcome| outcome.values[&target_force] == "high")
                .map(|outcome| outcome.probability)
                .try_fold(Rational::zero(), |sum, probability| {
                    sum.checked_add(probability)
                })
                .unwrap();
            assert_eq!(high_mass, Rational::new(11, 16).unwrap(), "leg {leg}");
            assert!(
                marginal
                    .outcomes
                    .iter()
                    .all(|outcome| { outcome.values[&other_contact] == "free" })
            );
            assert!(marginal.stats.peak_intermediate_scope_size <= 2);
            assert_eq!(marginal.stats.instantiated_factor_cells, 44);
        }
    }

    #[test]
    fn parallel_descending_pathways_remain_separate_and_position_gain_is_preserved() {
        let compiled = parallel_descending_gate_fixture().compile().unwrap();
        let current = parallel_pathway_current_state();

        let excitatory = compiled
            .marginal_next(
                &current,
                "drive-excitatory-pathway",
                &[
                    "descending-excitatory".to_owned(),
                    "descending-inhibitory".to_owned(),
                    "local-9a-activity".to_owned(),
                    "hook-movement-gain".to_owned(),
                    "claw-position-gain".to_owned(),
                ],
            )
            .unwrap();
        let inhibitory = compiled
            .marginal_next(
                &current,
                "drive-inhibitory-pathway",
                &[
                    "descending-excitatory".to_owned(),
                    "descending-inhibitory".to_owned(),
                    "local-9a-activity".to_owned(),
                    "hook-movement-gain".to_owned(),
                    "claw-position-gain".to_owned(),
                ],
            )
            .unwrap();

        assert_eq!(excitatory.outcomes.len(), 1);
        let excitatory_state = &excitatory.outcomes[0].values;
        assert_eq!(excitatory_state["descending-excitatory"], "high");
        assert_eq!(excitatory_state["descending-inhibitory"], "low");
        assert_eq!(excitatory_state["local-9a-activity"], "active");
        assert_eq!(excitatory_state["hook-movement-gain"], "suppressed");
        assert_eq!(excitatory_state["claw-position-gain"], "available");

        assert_eq!(inhibitory.outcomes.len(), 1);
        let inhibitory_state = &inhibitory.outcomes[0].values;
        assert_eq!(inhibitory_state["descending-excitatory"], "low");
        assert_eq!(inhibitory_state["descending-inhibitory"], "high");
        assert_eq!(inhibitory_state["local-9a-activity"], "inactive");
        assert_eq!(inhibitory_state["hook-movement-gain"], "available");
        assert_eq!(inhibitory_state["claw-position-gain"], "available");

        assert_eq!(excitatory.stats.peak_intermediate_scope_size, 3);
        assert_eq!(inhibitory.stats.peak_intermediate_scope_size, 3);
    }

    #[test]
    fn collapsed_single_pathway_ablation_makes_a_different_held_out_prediction() {
        let full_spec = parallel_descending_gate_fixture();
        let mut collapsed_spec = full_spec.clone();
        // The collapsed ablation makes the local gate independent of the
        // inhibitory pathway. The inhibitory action still changes its own
        // declared target, but no longer claims the disconnected local-gate
        // and hook variables as reachable descendants.
        let inhibitory_action = collapsed_spec
            .actions
            .iter_mut()
            .find(|action| action.id == "drive-inhibitory-pathway")
            .unwrap();
        inhibitory_action.allowed_descendant_variable_ids.clear();
        for gate in collapsed_spec
            .factors
            .iter_mut()
            .filter(|factor| factor.target_variable_id == "local-9a-activity")
        {
            gate.parents = vec![parent(ParentScope::Next, "descending-excitatory")];
            gate.rows = vec![
                row(
                    &[("descending-excitatory", "high")],
                    &[("active", 1, 1), ("inactive", 0, 1)],
                ),
                row(
                    &[("descending-excitatory", "low")],
                    &[("active", 0, 1), ("inactive", 1, 1)],
                ),
            ];
        }

        // In the collapsed model the inhibitory intervention no longer has
        // a local-gate or hook-channel effect. These variables can still
        // evolve as descendants of excitation under the excitatory action.
        for variable_id in ["local-9a-activity", "hook-movement-gain"] {
            let factor = collapsed_spec
                .factors
                .iter_mut()
                .find(|factor| {
                    factor.action_id == "drive-inhibitory-pathway"
                        && factor.target_variable_id == variable_id
                })
                .unwrap();
            *factor = identity_factor_for(
                "drive-inhibitory-pathway",
                variable_id,
                if variable_id == "local-9a-activity" {
                    &["active", "inactive"]
                } else {
                    &["available", "suppressed"]
                },
            );
        }

        let full = full_spec.compile().unwrap();
        let collapsed = collapsed_spec.compile().unwrap();
        let mut current = parallel_pathway_current_state();
        current.insert("descending-excitatory".to_owned(), "low".to_owned());
        current.insert("descending-inhibitory".to_owned(), "high".to_owned());

        let full_prediction = full
            .marginal_next(
                &current,
                "drive-excitatory-pathway",
                &[
                    "local-9a-activity".to_owned(),
                    "hook-movement-gain".to_owned(),
                    "claw-position-gain".to_owned(),
                ],
            )
            .unwrap();
        let collapsed_prediction = collapsed
            .marginal_next(
                &current,
                "drive-excitatory-pathway",
                &[
                    "local-9a-activity".to_owned(),
                    "hook-movement-gain".to_owned(),
                    "claw-position-gain".to_owned(),
                ],
            )
            .unwrap();

        assert_eq!(full_prediction.outcomes.len(), 1);
        assert_eq!(
            full_prediction.outcomes[0].values["local-9a-activity"],
            "inactive"
        );
        assert_eq!(
            full_prediction.outcomes[0].values["hook-movement-gain"],
            "available"
        );
        assert_eq!(
            full_prediction.outcomes[0].values["claw-position-gain"],
            "available"
        );

        assert_eq!(collapsed_prediction.outcomes.len(), 1);
        assert_eq!(
            collapsed_prediction.outcomes[0].values["local-9a-activity"],
            "active"
        );
        assert_eq!(
            collapsed_prediction.outcomes[0].values["hook-movement-gain"],
            "suppressed"
        );
        assert_ne!(full_prediction, collapsed_prediction);
    }

    #[test]
    fn current_state_assignment_must_be_complete_and_domain_valid() {
        let compiled = fixture().compile().unwrap();
        let invalid = BTreeMap::from([
            ("context".to_owned(), "walking".to_owned()),
            ("left-contact".to_owned(), "unknown".to_owned()),
            ("right-contact".to_owned(), "free".to_owned()),
        ]);
        assert!(matches!(
            compiled.marginal_next(&invalid, "perturb-left", &["left-contact".to_owned()]),
            Err(FactorizedError::InvalidAssignment(_))
        ));
    }
}
