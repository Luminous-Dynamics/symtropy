// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A small renderer-independent representation and interpreter for executable experiment math.
//!
//! This crate intentionally starts with acyclic arithmetic graphs. It validates the graph and
//! physical dimensions before evaluating it; it is not a general-purpose ODE solver or physics
//! engine. Stateful dynamics and domain-specific solvers belong behind later, explicit adapters.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "0.1";

/// One experiment with evidence references, typed variables, and a directed equation graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperimentGraph {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub question: String,
    pub kind: ExperimentKind,
    #[serde(default)]
    pub evidence: Vec<ModelEvidence>,
    pub variables: Vec<ModelVariable>,
    pub equations: Vec<EquationNode>,
    /// Explicit assumptions and current limitations; an empty list is not a claim of completeness.
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentKind {
    SyntheticBenchmark,
    HistoricalReconstruction,
    LaboratoryExperiment,
    ComputationalStudy,
    Counterfactual,
}

/// Evidence anchors are identifiers and locators, not proof that a source supports an equation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEvidence {
    pub id: String,
    pub uri: String,
    pub locator: String,
    pub note: String,
}

/// Semantic dimensions in order: length, mass, time, electric current, temperature,
/// amount of substance, luminous intensity, and angle.
///
/// The eighth axis is a **semantic angle type** for model checking. SI treats plane angle as
/// dimensionless; this graph keeps it distinct to catch mistakes such as adding radians to a
/// tooth count. It is a type-checking convention, not a claim that SI defines a separate base
/// dimension for angle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimension {
    pub exponents: [i8; 8],
}

impl Dimension {
    pub const DIMENSIONLESS: Self = Self { exponents: [0; 8] };

    pub const ANGLE: Self = Self {
        exponents: [0, 0, 0, 0, 0, 0, 0, 1],
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableRole {
    Parameter,
    Input,
    State,
    Observation,
    Derived,
    Output,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelVariable {
    pub id: String,
    /// Compact symbol used by the visual math view, e.g. theta_in.
    pub symbol: String,
    pub description: String,
    pub unit: String,
    pub dimension: Dimension,
    pub role: VariableRole,
    /// Initial/input value as a finite numeric string. Derived/output values are computed.
    #[serde(default)]
    pub value_decimal: Option<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquationOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquationBasis {
    SyntheticBenchmark,
    EstablishedLaw,
    SourceReported,
    Hypothesis,
    Approximation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquationNode {
    pub id: String,
    pub operator: EquationOperator,
    pub input_variable_ids: Vec<String>,
    pub output_variable_id: String,
    /// Formula text is generated from the typed operator and variable symbols, not stored separately.
    pub basis: EquationBasis,
    pub basis_note: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

fn variable_symbol<'a>(graph: &'a ExperimentGraph, id: &str) -> Result<&'a str, String> {
    graph
        .variables
        .iter()
        .find(|variable| variable.id == id)
        .map(|variable| variable.symbol.as_str())
        .ok_or_else(|| format!("unknown variable '{id}'"))
}

impl EquationNode {
    /// Derive a displayable formula from the executable operator graph.
    ///
    /// This prevents a manually typed equation caption from disagreeing with the operator
    /// that the interpreter actually executes. Renderers may typeset the returned expression,
    /// but the operation and variable references remain authoritative.
    pub fn canonical_expression(&self, graph: &ExperimentGraph) -> Result<String, String> {
        let output = variable_symbol(graph, &self.output_variable_id)?;
        let inputs: Vec<&str> = self
            .input_variable_ids
            .iter()
            .map(|id| variable_symbol(graph, id))
            .collect::<Result<_, _>>()?;

        match (self.operator, inputs.as_slice()) {
            (EquationOperator::Negate, [input]) => {
                Ok(format!("{output} = -({input})"))
            }
            (EquationOperator::Add, [left, right]) => {
                Ok(format!("{output} = {left} + {right}"))
            }
            (EquationOperator::Subtract, [left, right]) => {
                Ok(format!("{output} = {left} - {right}"))
            }
            (EquationOperator::Multiply, [left, right]) => {
                Ok(format!("{output} = {left} * {right}"))
            }
            (EquationOperator::Divide, [left, right]) => {
                Ok(format!("{output} = {left} / {right}"))
            }
            _ => Err(format!(
                "equation '{}' has input arity incompatible with its operator",
                self.id
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCode {
    EmptyId,
    DuplicateId,
    EmptyField,
    UnsupportedSchemaVersion,
    MissingEvidenceReference,
    InvalidNumericValue,
    MissingVariableReference,
    InvalidInputArity,
    DimensionMismatch,
    InvalidOutputRole,
    DuplicateEquationOutput,
    DependencyCycle,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub record_id: String,
    pub code: ValidationCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

fn issue(
    issues: &mut Vec<ValidationIssue>,
    record_id: impl Into<String>,
    code: ValidationCode,
    message: impl Into<String>,
) {
    issues.push(ValidationIssue {
        record_id: record_id.into(),
        code,
        message: message.into(),
    });
}

fn register_id(id: &str, seen: &mut BTreeSet<String>, issues: &mut Vec<ValidationIssue>) {
    if id.trim().is_empty() {
        issue(issues, "<empty>", ValidationCode::EmptyId, "record ID must not be empty");
    } else if !seen.insert(id.to_owned()) {
        issue(
            issues,
            id,
            ValidationCode::DuplicateId,
            "record ID is already used by another record in this experiment graph",
        );
    }
}

fn require_nonempty(
    value: &str,
    record_id: &str,
    field: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if value.trim().is_empty() {
        issue(
            issues,
            record_id,
            ValidationCode::EmptyField,
            format!("{field} must not be empty"),
        );
    }
}

fn parse_finite(value: &str) -> Option<f64> {
    if value.trim() != value || value.is_empty() {
        return None;
    }
    value.parse::<f64>().ok().filter(|number| number.is_finite())
}

fn combined_dimension(left: Dimension, right: Dimension, subtract: bool) -> Option<Dimension> {
    let mut exponents = [0_i8; 8];
    for (index, result) in exponents.iter_mut().enumerate() {
        let left = i16::from(left.exponents[index]);
        let right = i16::from(right.exponents[index]);
        let combined = if subtract { left - right } else { left + right };
        if combined < i16::from(i8::MIN) || combined > i16::from(i8::MAX) {
            return None;
        }
        *result = combined as i8;
    }
    Some(Dimension { exponents })
}

impl ExperimentGraph {
    /// Structural and dimensional validation. This does not establish the scientific truth of
    /// an equation or the semantic entailment of its evidence.
    pub fn validate(&self) -> ValidationReport {
        let mut report = ValidationReport::default();
        let issues = &mut report.issues;
        let mut all_ids = BTreeSet::new();

        if self.schema_version != SCHEMA_VERSION {
            issue(
                issues,
                &self.id,
                ValidationCode::UnsupportedSchemaVersion,
                format!(
                    "schema_version '{}' is unsupported; expected '{}'",
                    self.schema_version, SCHEMA_VERSION
                ),
            );
        }
        register_id(&self.id, &mut all_ids, issues);
        require_nonempty(&self.title, &self.id, "title", issues);
        require_nonempty(&self.question, &self.id, "question", issues);
        if self.limitations.is_empty() || self.limitations.iter().any(|item| item.trim().is_empty()) {
            issue(
                issues,
                &self.id,
                ValidationCode::EmptyField,
                "at least one explicit, non-empty limitation is required",
            );
        }
        if self.variables.is_empty() || self.equations.is_empty() {
            issue(
                issues,
                &self.id,
                ValidationCode::EmptyField,
                "an experiment graph needs at least one variable and one equation node",
            );
        }
        if self.kind == ExperimentKind::HistoricalReconstruction && self.evidence.is_empty() {
            issue(
                issues,
                &self.id,
                ValidationCode::MissingEvidenceReference,
                "historical reconstructions require at least one evidence anchor",
            );
        }

        let mut evidence_ids = BTreeSet::new();
        for evidence in &self.evidence {
            register_id(&evidence.id, &mut all_ids, issues);
            evidence_ids.insert(evidence.id.clone());
            require_nonempty(&evidence.uri, &evidence.id, "evidence URI", issues);
            require_nonempty(&evidence.locator, &evidence.id, "evidence locator", issues);
            require_nonempty(&evidence.note, &evidence.id, "evidence note", issues);
        }

        let mut variables = BTreeMap::new();
        for variable in &self.variables {
            register_id(&variable.id, &mut all_ids, issues);
            require_nonempty(&variable.symbol, &variable.id, "variable symbol", issues);
            require_nonempty(&variable.description, &variable.id, "variable description", issues);
            require_nonempty(&variable.unit, &variable.id, "variable unit", issues);
            if let Some(value) = &variable.value_decimal {
                if parse_finite(value).is_none() {
                    issue(
                        issues,
                        &variable.id,
                        ValidationCode::InvalidNumericValue,
                        "value_decimal must be a finite numeric string",
                    );
                }
                if matches!(variable.role, VariableRole::Derived | VariableRole::Output) {
                    issue(
                        issues,
                        &variable.id,
                        ValidationCode::InvalidNumericValue,
                        "derived/output variables must not carry an initial value; their equation computes it",
                    );
                }
            }
            for evidence_id in &variable.evidence_ids {
                if !evidence_ids.contains(evidence_id) {
                    issue(
                        issues,
                        &variable.id,
                        ValidationCode::MissingEvidenceReference,
                        format!("variable refers to unknown evidence ID '{evidence_id}'"),
                    );
                }
            }
            variables.entry(variable.id.clone()).or_insert(variable);
        }

        let mut producers: BTreeMap<String, String> = BTreeMap::new();
        for node in &self.equations {
            register_id(&node.id, &mut all_ids, issues);
            require_nonempty(&node.basis_note, &node.id, "equation basis note", issues);

            if node.basis == EquationBasis::SourceReported && node.evidence_ids.is_empty() {
                issue(
                    issues,
                    &node.id,
                    ValidationCode::MissingEvidenceReference,
                    "source-reported equations require at least one evidence ID",
                );
            }
            for evidence_id in &node.evidence_ids {
                if !evidence_ids.contains(evidence_id) {
                    issue(
                        issues,
                        &node.id,
                        ValidationCode::MissingEvidenceReference,
                        format!("equation refers to unknown evidence ID '{evidence_id}'"),
                    );
                }
            }

            let arity_valid = match node.operator {
                EquationOperator::Negate => node.input_variable_ids.len() == 1,
                EquationOperator::Add
                | EquationOperator::Subtract
                | EquationOperator::Multiply
                | EquationOperator::Divide => node.input_variable_ids.len() == 2,
            };
            if !arity_valid {
                issue(
                    issues,
                    &node.id,
                    ValidationCode::InvalidInputArity,
                    "this operator requires exactly one input (negate) or two inputs (binary arithmetic)",
                );
            }

            let output = variables.get(&node.output_variable_id).copied();
            match output {
                None => issue(
                    issues,
                    &node.id,
                    ValidationCode::MissingVariableReference,
                    format!("equation refers to unknown output variable '{}'", node.output_variable_id),
                ),
                Some(variable) => {
                    if !matches!(variable.role, VariableRole::Derived | VariableRole::Output) {
                        issue(
                            issues,
                            &node.id,
                            ValidationCode::InvalidOutputRole,
                            "equation output variable must have derived or output role",
                        );
                    }
                    if variable.value_decimal.is_some() {
                        issue(
                            issues,
                            &node.id,
                            ValidationCode::InvalidNumericValue,
                            "equation output variable must not have an initial value",
                        );
                    }
                }
            }

            match producers.get(&node.output_variable_id) {
                Some(previous) => issue(
                    issues,
                    &node.id,
                    ValidationCode::DuplicateEquationOutput,
                    format!(
                        "output variable '{}' is already produced by equation '{previous}'",
                        node.output_variable_id
                    ),
                ),
                None => {
                    if output.is_some() {
                        producers.insert(node.output_variable_id.clone(), node.id.clone());
                    }
                }
            }

            let input_variables: Option<Vec<&ModelVariable>> = node
                .input_variable_ids
                .iter()
                .map(|id| variables.get(id).copied())
                .collect();
            match input_variables {
                None => {
                    for input_id in &node.input_variable_ids {
                        if !variables.contains_key(input_id) {
                            issue(
                                issues,
                                &node.id,
                                ValidationCode::MissingVariableReference,
                                format!("equation refers to unknown input variable '{input_id}'"),
                            );
                        }
                    }
                }
                Some(input_variables) if arity_valid && output.is_some() => {
                    let output_dimension = output.expect("checked above").dimension;
                    let expected_dimension = match node.operator {
                        EquationOperator::Add | EquationOperator::Subtract => {
                            if input_variables[0].dimension == input_variables[1].dimension {
                                Some(input_variables[0].dimension)
                            } else {
                                None
                            }
                        }
                        EquationOperator::Multiply => combined_dimension(
                            input_variables[0].dimension,
                            input_variables[1].dimension,
                            false,
                        ),
                        EquationOperator::Divide => combined_dimension(
                            input_variables[0].dimension,
                            input_variables[1].dimension,
                            true,
                        ),
                        EquationOperator::Negate => Some(input_variables[0].dimension),
                    };
                    if expected_dimension != Some(output_dimension) {
                        issue(
                            issues,
                            &node.id,
                            ValidationCode::DimensionMismatch,
                            "operator input dimensions are incompatible with the output variable dimension",
                        );
                    }
                }
                Some(_) => {}
            }
        }

        // Kahn-style dependency check: equations can be stored in any order, but a runnable
        // arithmetic graph must not contain an implicit feedback loop.
        let mut pending: BTreeSet<String> =
            self.equations.iter().map(|node| node.id.clone()).collect();
        let mut completed = BTreeSet::new();
        loop {
            let ready: Vec<String> = self
                .equations
                .iter()
                .filter(|node| pending.contains(&node.id))
                .filter(|node| {
                    node.input_variable_ids.iter().all(|input_id| {
                        producers
                            .get(input_id)
                            .map(|producer_id| completed.contains(producer_id))
                            .unwrap_or(true)
                    })
                })
                .map(|node| node.id.clone())
                .collect();
            if ready.is_empty() {
                break;
            }
            for id in ready {
                pending.remove(&id);
                completed.insert(id);
            }
        }
        for node_id in pending {
            issue(
                issues,
                node_id,
                ValidationCode::DependencyCycle,
                "equation dependencies contain a cycle; represent stateful/time-stepped systems explicitly rather than as an implicit arithmetic loop",
            );
        }

        issues.sort();
        report
    }

    /// Execute a validated acyclic arithmetic graph using IEEE-754 binary64 operations.
    ///
    /// This is a minimal reference interpreter for checking graph semantics. It is not a
    /// high-precision arithmetic engine, an ODE solver, or physical validation. Scientific
    /// backends must declare their solver and numerical tolerances separately.
    pub fn evaluate(&self) -> Result<BTreeMap<String, String>, String> {
        let report = self.validate();
        if !report.is_valid() {
            return Err(format!("experiment graph is invalid: {:?}", report.issues));
        }

        let mut values: BTreeMap<String, f64> = BTreeMap::new();
        for variable in &self.variables {
            if let Some(value) = &variable.value_decimal {
                let number = parse_finite(value)
                    .ok_or_else(|| format!("invalid value '{}' for '{}'", value, variable.id))?;
                values.insert(variable.id.clone(), number);
            }
        }

        let mut pending: Vec<&EquationNode> = self.equations.iter().collect();
        while !pending.is_empty() {
            let ready_index = pending.iter().position(|node| {
                node.input_variable_ids.iter().all(|id| values.contains_key(id))
            });
            let Some(index) = ready_index else {
                let missing: Vec<&str> = pending
                    .iter()
                    .flat_map(|node| node.input_variable_ids.iter().map(String::as_str))
                    .filter(|id| !values.contains_key(*id))
                    .collect();
                return Err(format!(
                    "evaluation stalled: missing runtime inputs or unresolved dependencies: {:?}",
                    missing
                ));
            };
            let node = pending.remove(index);
            let args: Vec<f64> = node
                .input_variable_ids
                .iter()
                .map(|id| values.get(id).copied().expect("readiness checked"))
                .collect();
            let result = match node.operator {
                EquationOperator::Add => args[0] + args[1],
                EquationOperator::Subtract => args[0] - args[1],
                EquationOperator::Multiply => args[0] * args[1],
                EquationOperator::Divide => {
                    if args[1] == 0.0 {
                        return Err(format!("division by zero in equation '{}'", node.id));
                    }
                    args[0] / args[1]
                }
                EquationOperator::Negate => -args[0],
            };
            if !result.is_finite() {
                return Err(format!("non-finite result in equation '{}'", node.id));
            }
            values.insert(node.output_variable_id.clone(), result);
        }

        Ok(values
            .into_iter()
            .map(|(id, value)| (id, value.to_string()))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gear_graph() -> ExperimentGraph {
        serde_json::from_str(include_str!("../fixtures/gear-ratio.json"))
            .expect("parse synthetic gear-ratio experiment")
    }

    #[test]
    fn canonical_formula_labels_are_derived_from_executable_operators() {
        let graph = gear_graph();
        let ratio = graph
            .equations
            .iter()
            .find(|node| node.id == "equation:gear-ratio")
            .expect("gear ratio equation");
        assert_eq!(
            ratio.canonical_expression(&graph).unwrap(),
            "r = N_driver / N_driven"
        );
        let reverse = graph
            .equations
            .iter()
            .find(|node| node.id == "equation:external-gear-direction")
            .expect("direction equation");
        assert_eq!(
            reverse.canonical_expression(&graph).unwrap(),
            "r_signed = -(r)"
        );
    }

    #[test]
    fn subtraction_operator_executes_and_renders() {
        let mut graph = gear_graph();
        let phase = graph
            .equations
            .iter_mut()
            .find(|node| node.id == "equation:phase-offset")
            .expect("phase equation");
        phase.operator = EquationOperator::Subtract;
        assert_eq!(
            phase.canonical_expression(&graph).unwrap(),
            "theta_out = theta_scaled - phi"
        );
        let values = graph.evaluate().expect("execute subtraction variant");
        let output = values["variable:theta_output"].parse::<f64>().unwrap();
        assert!((output - (-2.1_f64)).abs() < 1e-12);
    }

    #[test]
    fn dimension_exponent_overflow_is_rejected() {
        let extreme = Dimension {
            exponents: [i8::MAX, 0, 0, 0, 0, 0, 0, 0],
        };
        assert!(combined_dimension(extreme, extreme, false).is_none());
        assert!(combined_dimension(extreme, Dimension::DIMENSIONLESS, true).is_some());
    }

    #[test]
    fn duplicate_equation_outputs_are_rejected() {
        let mut graph = gear_graph();
        let output_id = graph.equations[0].output_variable_id.clone();
        graph.equations[1].output_variable_id = output_id;
        let report = graph.validate();
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::DuplicateEquationOutput
        }));
    }

    #[test]
    fn gear_ratio_graph_evaluates_to_known_answer() {
        let graph = gear_graph();
        let report = graph.validate();
        assert!(report.is_valid(), "{:#?}", report.issues);
        let values = graph.evaluate().expect("execute gear-ratio graph");
        let output = values["variable:theta_output"].parse::<f64>().unwrap();
        assert!((output - (-1.9_f64)).abs() < 1e-12);
        let ratio = values["variable:signed_ratio"].parse::<f64>().unwrap();
        assert!((ratio - (-4.0_f64)).abs() < 1e-12);
    }

    #[test]
    fn incompatible_dimensions_are_rejected() {
        let mut graph = gear_graph();
        let output = graph
            .variables
            .iter_mut()
            .find(|variable| variable.id == "variable:theta_scaled")
            .expect("scaled angle variable");
        output.dimension.exponents[2] = 1;
        let report = graph.validate();
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::DimensionMismatch
        }));
    }

    #[test]
    fn cyclic_dependencies_are_rejected() {
        let mut graph = gear_graph();
        graph.equations[0].input_variable_ids =
            vec!["variable:signed_ratio".to_owned(), "variable:n_driven".to_owned()];
        let report = graph.validate();
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::DependencyCycle
        }));
    }

    #[test]
    fn missing_runtime_input_fails_closed() {
        let mut graph = gear_graph();
        let input = graph
            .variables
            .iter_mut()
            .find(|variable| variable.id == "variable:n_driver")
            .expect("driver tooth count");
        input.value_decimal = None;
        assert!(graph.evaluate().is_err());
    }

    #[test]
    fn division_by_zero_fails_closed() {
        let mut graph = gear_graph();
        let input = graph
            .variables
            .iter_mut()
            .find(|variable| variable.id == "variable:n_driven")
            .expect("driven tooth count");
        input.value_decimal = Some("0".to_owned());
        assert!(graph.evaluate().is_err());
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut graph = gear_graph();
        let duplicate_id = graph.variables[0].id.clone();
        graph.variables[1].id = duplicate_id;
        let report = graph.validate();
        assert!(report.issues.iter().any(|item| {
            item.code == ValidationCode::DuplicateId
        }));
    }
}
