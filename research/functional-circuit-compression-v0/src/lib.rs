//! Exhaustive synthetic oracle for intervention-predictive circuit compression.
//!
//! Exactness is limited to a frozen finite component set and its supplied
//! additive/higher-order contribution terms. This is not a causal estimator,
//! connectome parser, neural simulator, or biological sufficiency proof.

use std::collections::{BTreeMap, BTreeSet};

const MAX_EXACT_COMPONENTS: usize = 20;
const FLOAT_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComponentKind {
    Synapse,
    Neuron,
    Pathway,
    SensoryChannel,
    DescendingInput,
    MotorModule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentEvidence {
    SourceQualified,
    Uncertain,
    Unobserved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitComponent {
    pub component_id: String,
    pub source_record_id: String,
    pub kind: ComponentKind,
    pub evidence: ComponentEvidence,
    /// Components that must remain present for this component to be retained.
    /// A synapse normally requires both its pre- and postsynaptic neuron IDs.
    pub requires: Vec<String>,
    /// Comparable complexity units declared by the problem manifest.
    pub complexity_cost: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationScenario {
    pub scenario_id: String,
    pub output_ids: Vec<String>,
    /// Per-output absolute non-inferiority tolerances, in the output's units.
    pub absolute_tolerances: Vec<f64>,
}

/// A frozen model-response contribution active only when every required
/// component is retained. An empty required set is a shared context term.
#[derive(Debug, Clone, PartialEq)]
pub struct ContributionTerm {
    pub scenario_id: String,
    pub required_components: Vec<String>,
    pub output_delta: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressionProblem {
    pub complexity_unit_id: String,
    pub source_coverage_complete: bool,
    pub components: Vec<CircuitComponent>,
    pub scenarios: Vec<EvaluationScenario>,
    pub terms: Vec<ContributionTerm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompressionConfig {
    pub maximum_components: usize,
    pub maximum_subset_evaluations: u64,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            maximum_components: MAX_EXACT_COMPONENTS,
            maximum_subset_evaluations: 1_u64 << MAX_EXACT_COMPONENTS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompressionError {
    EmptyIdentifier(&'static str),
    EmptyComponentSet,
    EmptyScenarioSet,
    EmptyOutputSet(String),
    DuplicateComponent(String),
    DuplicateScenario(String),
    DuplicateOutput {
        scenario_id: String,
        output_id: String,
    },
    DuplicateDependency {
        component_id: String,
        dependency_id: String,
    },
    SelfDependency(String),
    UnknownComponent(String),
    UnknownScenario(String),
    CostUnitMissing,
    TooManyComponents {
        actual: usize,
        maximum: usize,
    },
    InvalidComponentCost(String),
    InvalidTolerance {
        scenario_id: String,
        output_id: String,
    },
    OutputDimensionMismatch {
        scenario_id: String,
    },
    NonFiniteContribution {
        scenario_id: String,
    },
    NonFiniteArithmetic,
    ComplexityCostOverflow,
    InvalidSearchBudget,
    SearchBudgetInsufficient {
        required: u64,
        provided: u64,
    },
    NoSufficientCircuit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstentionReason {
    SourceCoverageIncomplete,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompressionOutcome {
    Abstain {
        reason: AbstentionReason,
        source_coverage_complete: bool,
        unqualified_component_ids: Vec<String>,
    },
    Certified(CompressionReport),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputPrediction {
    pub output_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioPrediction {
    pub scenario_id: String,
    pub outputs: Vec<OutputPrediction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinimalCircuit {
    pub component_ids: Vec<String>,
    pub source_record_ids: Vec<String>,
    pub component_count: usize,
    pub total_complexity_cost: u64,
    /// Maximum |candidate - full model| / declared absolute tolerance.
    pub worst_normalized_error: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompressionReport {
    pub complexity_unit_id: String,
    pub source_coverage_complete: bool,
    pub component_total: usize,
    pub scenario_total: usize,
    pub exhaustive_subsets_considered: u64,
    pub structurally_valid_subsets: u64,
    pub sufficient_subsets: u64,
    /// All inclusion-minimal sufficient circuits, not one arbitrary winner.
    pub minimal_circuits: Vec<MinimalCircuit>,
    /// Full-model reference outputs used to evaluate every subset.
    pub reference_predictions: Vec<ScenarioPrediction>,
}

#[derive(Debug, Clone)]
struct CompiledTerm {
    scenario_index: usize,
    required_mask: u64,
    output_delta: Vec<f64>,
}

fn validate_problem(
    problem: &CompressionProblem,
    config: &CompressionConfig,
) -> Result<(), CompressionError> {
    if problem.complexity_unit_id.trim().is_empty() {
        return Err(CompressionError::CostUnitMissing);
    }
    if problem.components.is_empty() {
        return Err(CompressionError::EmptyComponentSet);
    }
    if problem.scenarios.is_empty() {
        return Err(CompressionError::EmptyScenarioSet);
    }
    if config.maximum_components == 0 || config.maximum_subset_evaluations == 0 {
        return Err(CompressionError::InvalidSearchBudget);
    }

    let mut component_ids = BTreeSet::new();
    for component in &problem.components {
        if component.component_id.trim().is_empty() {
            return Err(CompressionError::EmptyIdentifier("component_id"));
        }
        if component.source_record_id.trim().is_empty() {
            return Err(CompressionError::EmptyIdentifier("source_record_id"));
        }
        if component.complexity_cost == 0 {
            return Err(CompressionError::InvalidComponentCost(
                component.component_id.clone(),
            ));
        }
        if !component_ids.insert(component.component_id.clone()) {
            return Err(CompressionError::DuplicateComponent(
                component.component_id.clone(),
            ));
        }
    }

    for component in &problem.components {
        let mut dependencies = BTreeSet::new();
        for dependency in &component.requires {
            if dependency == &component.component_id {
                return Err(CompressionError::SelfDependency(
                    component.component_id.clone(),
                ));
            }
            if !dependencies.insert(dependency) {
                return Err(CompressionError::DuplicateDependency {
                    component_id: component.component_id.clone(),
                    dependency_id: dependency.clone(),
                });
            }
            if !component_ids.contains(dependency) {
                return Err(CompressionError::UnknownComponent(dependency.clone()));
            }
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut scenario_by_id = BTreeMap::new();
    for (index, scenario) in problem.scenarios.iter().enumerate() {
        if scenario.scenario_id.trim().is_empty() {
            return Err(CompressionError::EmptyIdentifier("scenario_id"));
        }
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(CompressionError::DuplicateScenario(
                scenario.scenario_id.clone(),
            ));
        }
        if scenario.output_ids.is_empty() {
            return Err(CompressionError::EmptyOutputSet(
                scenario.scenario_id.clone(),
            ));
        }
        if scenario.output_ids.len() != scenario.absolute_tolerances.len() {
            return Err(CompressionError::OutputDimensionMismatch {
                scenario_id: scenario.scenario_id.clone(),
            });
        }
        let mut output_ids = BTreeSet::new();
        for (output_id, tolerance) in scenario
            .output_ids
            .iter()
            .zip(&scenario.absolute_tolerances)
        {
            if output_id.trim().is_empty() {
                return Err(CompressionError::EmptyIdentifier("output_id"));
            }
            if !output_ids.insert(output_id) {
                return Err(CompressionError::DuplicateOutput {
                    scenario_id: scenario.scenario_id.clone(),
                    output_id: output_id.clone(),
                });
            }
            if !tolerance.is_finite() || *tolerance <= 0.0 {
                return Err(CompressionError::InvalidTolerance {
                    scenario_id: scenario.scenario_id.clone(),
                    output_id: output_id.clone(),
                });
            }
        }
        scenario_by_id.insert(scenario.scenario_id.clone(), index);
    }

    for term in &problem.terms {
        let Some(&scenario_index) = scenario_by_id.get(&term.scenario_id) else {
            return Err(CompressionError::UnknownScenario(term.scenario_id.clone()));
        };
        let mut required = BTreeSet::new();
        for id in &term.required_components {
            if !component_ids.contains(id) {
                return Err(CompressionError::UnknownComponent(id.clone()));
            }
            if !required.insert(id) {
                return Err(CompressionError::DuplicateDependency {
                    component_id: term.scenario_id.clone(),
                    dependency_id: id.clone(),
                });
            }
        }
        let scenario = &problem.scenarios[scenario_index];
        if term.output_delta.len() != scenario.output_ids.len() {
            return Err(CompressionError::OutputDimensionMismatch {
                scenario_id: term.scenario_id.clone(),
            });
        }
        if term.output_delta.iter().any(|value| !value.is_finite()) {
            return Err(CompressionError::NonFiniteContribution {
                scenario_id: term.scenario_id.clone(),
            });
        }
    }

    Ok(())
}

fn dependency_closed(mask: u64, dependency_masks: &[u64]) -> bool {
    dependency_masks
        .iter()
        .enumerate()
        .all(|(index, required)| {
            let bit = 1_u64 << index;
            mask & bit == 0 || mask & required == *required
        })
}

fn add_checked(target: &mut [f64], delta: &[f64]) -> Result<(), CompressionError> {
    for (value, change) in target.iter_mut().zip(delta) {
        *value += change;
        if !value.is_finite() {
            return Err(CompressionError::NonFiniteArithmetic);
        }
    }
    Ok(())
}

/// Enumerate all dependency-closed component subsets and return every
/// inclusion-minimal subset preserving the full model's frozen intervention
/// response signatures within declared per-output tolerances.
///
/// Exactness is relative to the supplied finite contribution-term expansion.
/// It does not establish that those terms are biologically correct.
pub fn compress_exact(
    problem: &CompressionProblem,
    config: &CompressionConfig,
) -> Result<CompressionOutcome, CompressionError> {
    validate_problem(problem, config)?;

    let mut unqualified: Vec<String> = problem
        .components
        .iter()
        .filter(|component| component.evidence != ComponentEvidence::SourceQualified)
        .map(|component| component.component_id.clone())
        .collect();
    unqualified.sort();

    if !problem.source_coverage_complete || !unqualified.is_empty() {
        return Ok(CompressionOutcome::Abstain {
            reason: AbstentionReason::SourceCoverageIncomplete,
            source_coverage_complete: problem.source_coverage_complete,
            unqualified_component_ids: unqualified,
        });
    }

    let mut components: Vec<&CircuitComponent> = problem.components.iter().collect();
    components.sort_by(|left, right| left.component_id.cmp(&right.component_id));
    let component_count = components.len();
    let maximum = config.maximum_components.min(MAX_EXACT_COMPONENTS);
    if component_count > maximum {
        return Err(CompressionError::TooManyComponents {
            actual: component_count,
            maximum,
        });
    }

    let subset_count = 1_u64 << component_count;
    if config.maximum_subset_evaluations < subset_count {
        return Err(CompressionError::SearchBudgetInsufficient {
            required: subset_count,
            provided: config.maximum_subset_evaluations,
        });
    }

    let component_index: BTreeMap<String, usize> = components
        .iter()
        .enumerate()
        .map(|(index, component)| (component.component_id.clone(), index))
        .collect();

    let mut dependency_masks = vec![0_u64; component_count];
    for (index, component) in components.iter().enumerate() {
        for dependency in &component.requires {
            dependency_masks[index] |= 1_u64 << component_index[dependency];
        }
    }

    let mut scenarios: Vec<&EvaluationScenario> = problem.scenarios.iter().collect();
    scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let scenario_index: BTreeMap<String, usize> = scenarios
        .iter()
        .enumerate()
        .map(|(index, scenario)| (scenario.scenario_id.clone(), index))
        .collect();

    let mut compiled_terms = Vec::with_capacity(problem.terms.len());
    for term in &problem.terms {
        let required_mask = term
            .required_components
            .iter()
            .fold(0_u64, |mask, id| mask | (1_u64 << component_index[id]));
        compiled_terms.push(CompiledTerm {
            scenario_index: scenario_index[&term.scenario_id],
            required_mask,
            output_delta: term.output_delta.clone(),
        });
    }
    compiled_terms.sort_by(|left, right| {
        left.scenario_index
            .cmp(&right.scenario_index)
            .then_with(|| left.required_mask.cmp(&right.required_mask))
            .then_with(|| {
                left.output_delta
                    .iter()
                    .zip(&right.output_delta)
                    .find_map(|(a, b)| {
                        let ordering = a.total_cmp(b);
                        (ordering != std::cmp::Ordering::Equal).then_some(ordering)
                    })
                    .unwrap_or_else(|| left.output_delta.len().cmp(&right.output_delta.len()))
            })
    });

    let output_counts: Vec<usize> = scenarios
        .iter()
        .map(|scenario| scenario.output_ids.len())
        .collect();
    let mut reference: Vec<Vec<f64>> = output_counts
        .iter()
        .map(|count| vec![0.0; *count])
        .collect();
    for term in &compiled_terms {
        add_checked(&mut reference[term.scenario_index], &term.output_delta)?;
    }

    let mut sufficient = vec![false; subset_count as usize];
    let mut errors = vec![f64::INFINITY; subset_count as usize];
    let mut structurally_valid_subsets = 0_u64;
    let mut sufficient_subsets = 0_u64;

    for mask in 0..subset_count {
        if !dependency_closed(mask, &dependency_masks) {
            continue;
        }
        structurally_valid_subsets += 1;
        let mut prediction: Vec<Vec<f64>> = output_counts
            .iter()
            .map(|count| vec![0.0; *count])
            .collect();

        for term in &compiled_terms {
            if mask & term.required_mask == term.required_mask {
                add_checked(&mut prediction[term.scenario_index], &term.output_delta)?;
            }
        }

        let mut worst_normalized_error = 0.0_f64;
        let mut is_sufficient = true;
        for (index, scenario) in scenarios.iter().enumerate() {
            for ((predicted, expected), tolerance) in prediction[index]
                .iter()
                .zip(&reference[index])
                .zip(&scenario.absolute_tolerances)
            {
                let normalized_error = (predicted - expected).abs() / tolerance;
                if !normalized_error.is_finite() {
                    return Err(CompressionError::NonFiniteArithmetic);
                }
                worst_normalized_error = worst_normalized_error.max(normalized_error);
                if normalized_error > 1.0 + FLOAT_TOLERANCE {
                    is_sufficient = false;
                }
            }
        }
        errors[mask as usize] = worst_normalized_error;
        sufficient[mask as usize] = is_sufficient;
        if is_sufficient {
            sufficient_subsets += 1;
        }
    }

    if sufficient_subsets == 0 {
        return Err(CompressionError::NoSufficientCircuit);
    }

    let mut minimal_circuits = Vec::new();
    for mask in 0..subset_count {
        if !sufficient[mask as usize] {
            continue;
        }
        let mut has_sufficient_proper_subset = false;
        if mask != 0 {
            let mut subset = (mask - 1) & mask;
            loop {
                if sufficient[subset as usize] {
                    has_sufficient_proper_subset = true;
                    break;
                }
                if subset == 0 {
                    break;
                }
                subset = (subset - 1) & mask;
            }
        }
        if has_sufficient_proper_subset {
            continue;
        }

        let mut component_ids = Vec::new();
        let mut source_record_ids = Vec::new();
        let mut total_complexity_cost = 0_u64;
        for (index, component) in components.iter().enumerate() {
            if mask & (1_u64 << index) != 0 {
                component_ids.push(component.component_id.clone());
                source_record_ids.push(component.source_record_id.clone());
                total_complexity_cost = total_complexity_cost
                    .checked_add(component.complexity_cost)
                    .ok_or(CompressionError::ComplexityCostOverflow)?;
            }
        }
        minimal_circuits.push(MinimalCircuit {
            component_count: component_ids.len(),
            component_ids,
            source_record_ids,
            total_complexity_cost,
            worst_normalized_error: errors[mask as usize],
        });
    }

    minimal_circuits.sort_by(|left, right| {
        left.component_count
            .cmp(&right.component_count)
            .then_with(|| left.total_complexity_cost.cmp(&right.total_complexity_cost))
            .then_with(|| left.component_ids.cmp(&right.component_ids))
    });

    let reference_predictions = scenarios
        .iter()
        .enumerate()
        .map(|(index, scenario)| ScenarioPrediction {
            scenario_id: scenario.scenario_id.clone(),
            outputs: scenario
                .output_ids
                .iter()
                .enumerate()
                .map(|(output_index, output_id)| OutputPrediction {
                    output_id: output_id.clone(),
                    value: reference[index][output_index],
                })
                .collect(),
        })
        .collect();

    Ok(CompressionOutcome::Certified(CompressionReport {
        complexity_unit_id: problem.complexity_unit_id.clone(),
        source_coverage_complete: true,
        component_total: component_count,
        scenario_total: scenarios.len(),
        exhaustive_subsets_considered: subset_count,
        structurally_valid_subsets,
        sufficient_subsets,
        minimal_circuits,
        reference_predictions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn component(id: &str) -> CircuitComponent {
        CircuitComponent {
            component_id: id.to_owned(),
            source_record_id: format!("source-{id}"),
            kind: ComponentKind::Pathway,
            evidence: ComponentEvidence::SourceQualified,
            requires: Vec::new(),
            complexity_cost: 1,
        }
    }

    fn scenario(id: &str, outputs: &[&str], tolerances: &[f64]) -> EvaluationScenario {
        EvaluationScenario {
            scenario_id: id.to_owned(),
            output_ids: outputs.iter().map(|value| (*value).to_owned()).collect(),
            absolute_tolerances: tolerances.to_vec(),
        }
    }

    fn term(scenario_id: &str, required: &[&str], delta: &[f64]) -> ContributionTerm {
        ContributionTerm {
            scenario_id: scenario_id.to_owned(),
            required_components: required.iter().map(|value| (*value).to_owned()).collect(),
            output_delta: delta.to_vec(),
        }
    }

    fn problem(
        components: Vec<CircuitComponent>,
        scenarios: Vec<EvaluationScenario>,
        terms: Vec<ContributionTerm>,
    ) -> CompressionProblem {
        CompressionProblem {
            complexity_unit_id: "component-cost-v1".to_owned(),
            source_coverage_complete: true,
            components,
            scenarios,
            terms,
        }
    }

    fn certified(outcome: CompressionOutcome) -> CompressionReport {
        match outcome {
            CompressionOutcome::Certified(report) => report,
            other => panic!("expected a certificate, got {other:?}"),
        }
    }

    #[test]
    fn recovers_the_minimal_necessary_path_and_removes_irrelevant_component() {
        let input = problem(
            vec![component("a"), component("b"), component("irrelevant")],
            vec![scenario("walking", &["stride"], &[0.1])],
            vec![
                term("walking", &["a"], &[0.5]),
                term("walking", &["b"], &[0.5]),
                term("walking", &["irrelevant"], &[0.0]),
            ],
        );
        let report = certified(compress_exact(&input, &CompressionConfig::default()).unwrap());
        assert_eq!(report.minimal_circuits.len(), 1);
        assert_eq!(
            report.minimal_circuits[0].component_ids,
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert_eq!(report.minimal_circuits[0].worst_normalized_error, 0.0);
        assert_eq!(report.exhaustive_subsets_considered, 8);
    }

    #[test]
    fn reports_alternative_minimal_redundant_pathways() {
        let input = problem(
            vec![component("route-a"), component("route-b")],
            vec![scenario("walking", &["speed"], &[0.5])],
            vec![
                term("walking", &["route-a"], &[0.5]),
                term("walking", &["route-b"], &[0.5]),
            ],
        );
        let report = certified(compress_exact(&input, &CompressionConfig::default()).unwrap());
        let circuits: Vec<Vec<String>> = report
            .minimal_circuits
            .iter()
            .map(|item| item.component_ids.clone())
            .collect();
        assert_eq!(
            circuits,
            vec![vec!["route-a".to_owned()], vec!["route-b".to_owned()]]
        );
    }

    #[test]
    fn preserves_a_synergistic_pair_that_is_weak_in_isolation() {
        let input = problem(
            vec![component("a"), component("b"), component("noise")],
            vec![scenario("turning", &["curvature"], &[0.1])],
            vec![term("turning", &["a", "b"], &[1.0])],
        );
        let report = certified(compress_exact(&input, &CompressionConfig::default()).unwrap());
        assert_eq!(report.minimal_circuits.len(), 1);
        assert_eq!(
            report.minimal_circuits[0].component_ids,
            vec!["a".to_owned(), "b".to_owned()]
        );
    }

    #[test]
    fn retains_components_needed_in_different_behavioural_contexts() {
        let input = problem(
            vec![component("walk-loop"), component("turn-loop")],
            vec![
                scenario("walking", &["phase"], &[0.05]),
                scenario("turning", &["curvature"], &[0.05]),
            ],
            vec![
                term("walking", &["walk-loop"], &[1.0]),
                term("turning", &["turn-loop"], &[1.0]),
            ],
        );
        let report = certified(compress_exact(&input, &CompressionConfig::default()).unwrap());
        assert_eq!(
            report.minimal_circuits[0].component_ids,
            vec!["turn-loop".to_owned(), "walk-loop".to_owned()]
        );
    }

    #[test]
    fn synapse_cannot_survive_when_its_endpoint_neurons_are_removed() {
        let mut synapse = component("synapse-1");
        synapse.kind = ComponentKind::Synapse;
        synapse.requires = vec!["pre".to_owned(), "post".to_owned()];
        let input = problem(
            vec![component("pre"), component("post"), synapse],
            vec![scenario("reflex", &["motor-output"], &[0.01])],
            vec![term("reflex", &["synapse-1"], &[2.0])],
        );
        let report = certified(compress_exact(&input, &CompressionConfig::default()).unwrap());
        assert_eq!(
            report.minimal_circuits[0].component_ids,
            vec!["post".to_owned(), "pre".to_owned(), "synapse-1".to_owned()]
        );
    }

    #[test]
    fn abstains_when_source_coverage_or_component_evidence_is_incomplete() {
        let mut uncertain = component("uncertain-path");
        uncertain.evidence = ComponentEvidence::Uncertain;
        let mut input = problem(
            vec![component("known-path"), uncertain],
            vec![scenario("walking", &["phase"], &[0.1])],
            vec![term("walking", &["known-path"], &[1.0])],
        );
        let outcome = compress_exact(&input, &CompressionConfig::default()).unwrap();
        assert_eq!(
            outcome,
            CompressionOutcome::Abstain {
                reason: AbstentionReason::SourceCoverageIncomplete,
                source_coverage_complete: true,
                unqualified_component_ids: vec!["uncertain-path".to_owned()],
            }
        );

        input.components[1].evidence = ComponentEvidence::SourceQualified;
        input.source_coverage_complete = false;
        let outcome = compress_exact(&input, &CompressionConfig::default()).unwrap();
        assert!(matches!(
            outcome,
            CompressionOutcome::Abstain {
                source_coverage_complete: false,
                ..
            }
        ));
    }

    #[test]
    fn insufficient_search_budget_fails_closed_instead_of_returning_partial_minima() {
        let input = problem(
            vec![component("a"), component("b"), component("c")],
            vec![scenario("walking", &["phase"], &[0.1])],
            vec![term("walking", &["a"], &[1.0])],
        );
        let config = CompressionConfig {
            maximum_components: 3,
            maximum_subset_evaluations: 7,
        };
        assert_eq!(
            compress_exact(&input, &config),
            Err(CompressionError::SearchBudgetInsufficient {
                required: 8,
                provided: 7,
            })
        );
    }

    #[test]
    fn input_order_does_not_change_the_reported_minimal_circuit() {
        let mut input = problem(
            vec![component("b"), component("a"), component("irrelevant")],
            vec![scenario("walking", &["stride"], &[0.1])],
            vec![
                term("walking", &["b"], &[0.5]),
                term("walking", &["irrelevant"], &[0.0]),
                term("walking", &["a"], &[0.5]),
            ],
        );
        let original = certified(compress_exact(&input, &CompressionConfig::default()).unwrap());
        input.components.reverse();
        input.terms.reverse();
        let reordered = certified(compress_exact(&input, &CompressionConfig::default()).unwrap());
        assert_eq!(original.minimal_circuits, reordered.minimal_circuits);
        assert_eq!(
            original.reference_predictions,
            reordered.reference_predictions
        );
    }

    #[test]
    fn rejects_a_term_that_names_an_unknown_component() {
        let input = problem(
            vec![component("known")],
            vec![scenario("walking", &["phase"], &[0.1])],
            vec![term("walking", &["missing"], &[1.0])],
        );
        assert_eq!(
            compress_exact(&input, &CompressionConfig::default()),
            Err(CompressionError::UnknownComponent("missing".to_owned()))
        );
    }
}
