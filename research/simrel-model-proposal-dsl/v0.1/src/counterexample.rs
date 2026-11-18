//! Finite-grid counterexample search between two validated model proposals.
//!
//! The returned witness is the lexicographically first distinguishing input
//! on the caller-supplied finite grid. It is not a globally minimal input,
//! hidden-truth oracle, proof of model inadequacy, or physical experiment.

use crate::{
    Dimension, EvaluationBudget, ExternalBinding, ModelError, ModelRunV0,
    ValidatedModelProgramV0,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

pub const MAX_COUNTEREXAMPLE_GRID_CASES: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CounterexampleSearchConfigV1 {
    pub steps: u32,
    pub seed: u64,
    pub budget: EvaluationBudget,
    pub absolute_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservableDifferenceV1 {
    pub step: u32,
    pub observable_name: String,
    pub dimension: Dimension,
    pub left_value: f64,
    pub right_value: f64,
    pub absolute_difference: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterexampleWitnessV1 {
    /// Index after canonical sorting and deduplication of the input grid.
    pub canonical_grid_index: usize,
    pub bindings: Vec<ExternalBinding>,
    pub differences: Vec<ObservableDifferenceV1>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CounterexampleSearchReportV1 {
    pub left_model_fingerprint64: u64,
    pub right_model_fingerprint64: u64,
    pub tested_grid_cases: usize,
    pub seed: u64,
    pub steps: u32,
    pub absolute_tolerance: f64,
    pub witness: Option<CounterexampleWitnessV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterexampleSearchError {
    EmptyGrid,
    GridTooLarge,
    InvalidTolerance,
    ModelExecutionFailed {
        side: CandidateSide,
        error: ModelError,
    },
    ObservableSetMismatch,
    ObservableDimensionMismatch,
    NonFiniteDifference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateSide {
    Left,
    Right,
}

impl fmt::Display for CounterexampleSearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyGrid => write!(f, "counterexample input grid must not be empty"),
            Self::GridTooLarge => write!(f, "counterexample input grid exceeds the declared limit"),
            Self::InvalidTolerance => {
                write!(f, "counterexample tolerance must be finite and non-negative")
            }
            Self::ModelExecutionFailed { side, error } => {
                write!(f, "{side:?} proposal failed during counterexample execution: {error}")
            }
            Self::ObservableSetMismatch => {
                write!(f, "candidate proposals do not expose the same observable names")
            }
            Self::ObservableDimensionMismatch => {
                write!(f, "candidate proposals assign different dimensions to a shared observable")
            }
            Self::NonFiniteDifference => {
                write!(f, "observable difference overflowed the finite numeric domain")
            }
        }
    }
}

impl std::error::Error for CounterexampleSearchError {}

/// Return the first distinguishing case in canonical lexicographic order.
/// Binding order supplied by the caller does not affect the search result.
pub fn find_lexicographically_first_counterexample(
    left: &ValidatedModelProgramV0,
    right: &ValidatedModelProgramV0,
    input_grid: &[Vec<ExternalBinding>],
    config: CounterexampleSearchConfigV1,
) -> Result<CounterexampleSearchReportV1, CounterexampleSearchError> {
    if input_grid.is_empty() {
        return Err(CounterexampleSearchError::EmptyGrid);
    }
    if input_grid.len() > MAX_COUNTEREXAMPLE_GRID_CASES {
        return Err(CounterexampleSearchError::GridTooLarge);
    }
    if !config.absolute_tolerance.is_finite() || config.absolute_tolerance < 0.0 {
        return Err(CounterexampleSearchError::InvalidTolerance);
    }

    let mut canonical_grid: Vec<Vec<ExternalBinding>> = input_grid
        .iter()
        .map(|case| {
            let mut ordered = case.clone();
            ordered.sort_by_key(|binding| binding.variable);
            ordered
        })
        .collect();
    canonical_grid.sort_by(compare_cases);
    canonical_grid.dedup_by(|left_case, right_case| cases_equal(left_case, right_case));

    let left_outputs = observable_schema(left);
    let right_outputs = observable_schema(right);
    if left_outputs.keys().ne(right_outputs.keys()) {
        return Err(CounterexampleSearchError::ObservableSetMismatch);
    }
    for (name, left_dimension) in &left_outputs {
        if right_outputs.get(name) != Some(left_dimension) {
            return Err(CounterexampleSearchError::ObservableDimensionMismatch);
        }
    }

    let mut witness = None;
    let mut tested_grid_cases = 0_usize;
    for (grid_index, bindings) in canonical_grid.iter().enumerate() {
        let left_run = left
            .simulate(bindings, config.steps, config.seed, config.budget)
            .map_err(|error| CounterexampleSearchError::ModelExecutionFailed {
                side: CandidateSide::Left,
                error,
            })?;
        let right_run = right
            .simulate(bindings, config.steps, config.seed, config.budget)
            .map_err(|error| CounterexampleSearchError::ModelExecutionFailed {
                side: CandidateSide::Right,
                error,
            })?;
        tested_grid_cases += 1;

        let differences = compare_runs(
            &left_run,
            &right_run,
            config.absolute_tolerance,
        )?;
        if !differences.is_empty() {
            witness = Some(CounterexampleWitnessV1 {
                canonical_grid_index: grid_index,
                bindings: bindings.clone(),
                differences,
            });
            break;
        }
    }

    Ok(CounterexampleSearchReportV1 {
        left_model_fingerprint64: left.fingerprint64(),
        right_model_fingerprint64: right.fingerprint64(),
        tested_grid_cases,
        seed: config.seed,
        steps: config.steps,
        absolute_tolerance: config.absolute_tolerance,
        witness,
    })
}

fn observable_schema(
    model: &ValidatedModelProgramV0,
) -> BTreeMap<String, Dimension> {
    model
        .program()
        .variables
        .iter()
        .filter(|declaration| declaration.role == crate::VariableRole::Observable)
        .map(|declaration| (declaration.name.clone(), declaration.dimension))
        .collect()
}

fn compare_runs(
    left: &ModelRunV0,
    right: &ModelRunV0,
    tolerance: f64,
) -> Result<Vec<ObservableDifferenceV1>, CounterexampleSearchError> {
    if left.samples.len() != right.samples.len() {
        return Err(CounterexampleSearchError::ObservableSetMismatch);
    }
    let mut differences = Vec::new();
    for (left_step, right_step) in left.samples.iter().zip(right.samples.iter()) {
        if left_step.step != right_step.step
            || left_step.values.len() != right_step.values.len()
        {
            return Err(CounterexampleSearchError::ObservableSetMismatch);
        }
        let left_values: BTreeMap<&str, (&Dimension, f64)> = left_step
            .values
            .iter()
            .map(|value| {
                (
                    value.name.as_str(),
                    (&value.dimension, value.value),
                )
            })
            .collect();
        let right_values: BTreeMap<&str, (&Dimension, f64)> = right_step
            .values
            .iter()
            .map(|value| {
                (
                    value.name.as_str(),
                    (&value.dimension, value.value),
                )
            })
            .collect();
        if left_values.keys().ne(right_values.keys()) {
            return Err(CounterexampleSearchError::ObservableSetMismatch);
        }
        for (name, (left_dimension, left_value)) in &left_values {
            let (right_dimension, right_value) = right_values
                .get(name)
                .copied()
                .ok_or(CounterexampleSearchError::ObservableSetMismatch)?;
            if *left_dimension != right_dimension {
                return Err(CounterexampleSearchError::ObservableDimensionMismatch);
            }
            let absolute_difference = (*left_value - right_value).abs();
            if !absolute_difference.is_finite() {
                return Err(CounterexampleSearchError::NonFiniteDifference);
            }
            if absolute_difference > tolerance {
                differences.push(ObservableDifferenceV1 {
                    step: left_step.step,
                    observable_name: (*name).to_owned(),
                    dimension: **left_dimension,
                    left_value: *left_value,
                    right_value,
                    absolute_difference,
                });
            }
        }
    }
    Ok(differences)
}

fn compare_cases(left: &[ExternalBinding], right: &[ExternalBinding]) -> Ordering {
    for (left_binding, right_binding) in left.iter().zip(right.iter()) {
        let order = left_binding
            .variable
            .cmp(&right_binding.variable)
            .then_with(|| left_binding.value.total_cmp(&right_binding.value));
        if order != Ordering::Equal {
            return order;
        }
    }
    left.len().cmp(&right.len())
}

fn cases_equal(left: &[ExternalBinding], right: &[ExternalBinding]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right.iter()).all(|(left, right)| {
            left.variable == right.variable
                && left.value.to_bits() == right.value.to_bits()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Assignment, Expr, ModelProgramV0, ProposalClass, ValueRange, VariableDecl, VariableId,
        VariableRole, SCHEMA_VERSION_V0,
    };

    fn affine_program(include_offset: bool, output_id: u16) -> ModelProgramV0 {
        ModelProgramV0 {
            schema_version: SCHEMA_VERSION_V0,
            model_id: if include_offset {
                "affine_structure".to_owned()
            } else {
                "linear_structure".to_owned()
            },
            variables: vec![
                VariableDecl {
                    id: VariableId(1),
                    name: "x".to_owned(),
                    role: VariableRole::Input,
                    dimension: Dimension::LENGTH,
                    admissible_range: Some(ValueRange {
                        minimum: -10.0,
                        maximum: 10.0,
                    }),
                    initial_value: None,
                },
                VariableDecl {
                    id: VariableId(2),
                    name: "gain".to_owned(),
                    role: VariableRole::Parameter,
                    dimension: Dimension::DIMENSIONLESS,
                    admissible_range: Some(ValueRange {
                        minimum: 0.0,
                        maximum: 3.0,
                    }),
                    initial_value: None,
                },
                VariableDecl {
                    id: VariableId(3),
                    name: "offset".to_owned(),
                    role: VariableRole::Parameter,
                    dimension: Dimension::LENGTH,
                    admissible_range: Some(ValueRange {
                        minimum: -2.0,
                        maximum: 2.0,
                    }),
                    initial_value: None,
                },
                VariableDecl {
                    id: VariableId(output_id),
                    name: "y".to_owned(),
                    role: VariableRole::Observable,
                    dimension: Dimension::LENGTH,
                    admissible_range: None,
                    initial_value: None,
                },
            ],
            class: ProposalClass::Algebraic,
            transitions: Vec::new(),
            outputs: vec![Assignment {
                target: VariableId(output_id),
                expression: if include_offset {
                    Expr::add(
                        Expr::multiply(
                            Expr::variable(VariableId(1)),
                            Expr::variable(VariableId(2)),
                        ),
                        Expr::variable(VariableId(3)),
                    )
                } else {
                    Expr::multiply(
                        Expr::variable(VariableId(1)),
                        Expr::variable(VariableId(2)),
                    )
                },
            }],
            maximum_horizon_steps: 1,
        }
    }

    fn binding_x(x: f64) -> Vec<ExternalBinding> {
        vec![
            ExternalBinding {
                variable: VariableId(1),
                value: x,
            },
            ExternalBinding {
                variable: VariableId(2),
                value: 1.5,
            },
            ExternalBinding {
                variable: VariableId(3),
                value: 0.5,
            },
        ]
    }

    fn config() -> CounterexampleSearchConfigV1 {
        CounterexampleSearchConfigV1 {
            steps: 1,
            seed: 77,
            budget: EvaluationBudget {
                maximum_steps: 1,
                expression_fuel: 100,
            },
            absolute_tolerance: 1.0e-12,
        }
    }

    #[test]
    fn finds_lexicographically_first_distinguishing_intervention() {
        let left = ValidatedModelProgramV0::new(affine_program(false, 4)).unwrap();
        let right = ValidatedModelProgramV0::new(affine_program(true, 9)).unwrap();
        let grid = vec![binding_x(3.0), binding_x(0.0), binding_x(-2.0)];
        let report =
            find_lexicographically_first_counterexample(&left, &right, &grid, config()).unwrap();
        let witness = report.witness.unwrap();
        assert_eq!(witness.bindings[0].value, -2.0);
        assert_eq!(witness.canonical_grid_index, 0);
        assert_eq!(witness.differences[0].observable_name, "y");
        assert_eq!(witness.differences[0].absolute_difference, 0.5);
        assert_eq!(report.tested_grid_cases, 1);
    }

    #[test]
    fn reports_no_witness_when_proposals_are_equivalent_on_the_supplied_grid() {
        let left = ValidatedModelProgramV0::new(affine_program(0.0, 4)).unwrap();
        let right = ValidatedModelProgramV0::new(affine_program(true, 9)).unwrap();
        let mut cases = vec![binding_x(-1.0), binding_x(0.0), binding_x(1.0)];
        for case in &mut cases {
            case[2].value = 0.0;
        }
        let report =
            find_lexicographically_first_counterexample(&left, &right, &cases, config()).unwrap();
        assert!(report.witness.is_none());
        assert_eq!(report.tested_grid_cases, 3);
    }

    #[test]
    fn canonical_search_order_is_independent_of_input_grid_order() {
        let left = ValidatedModelProgramV0::new(affine_program(0.0, 4)).unwrap();
        let right = ValidatedModelProgramV0::new(affine_program(0.5, 9)).unwrap();
        let forward = vec![binding_x(2.0), binding_x(-3.0), binding_x(1.0)];
        let reverse = vec![binding_x(1.0), binding_x(-3.0), binding_x(2.0)];
        let first =
            find_lexicographically_first_counterexample(&left, &right, &forward, config()).unwrap();
        let second =
            find_lexicographically_first_counterexample(&left, &right, &reverse, config()).unwrap();
        assert_eq!(first.witness, second.witness);
    }

    #[test]
    fn rejects_incompatible_output_schemas_and_invalid_tolerances() {
        let left = ValidatedModelProgramV0::new(affine_program(0.0, 4)).unwrap();
        let mut different_schema = affine_program(0.5, 9);
        different_schema.variables[3].name = "different_y".to_owned();
        let right = ValidatedModelProgramV0::new(different_schema).unwrap();
        assert_eq!(
            find_lexicographically_first_counterexample(
                &left,
                &right,
                &[binding_x(1.0)],
                config()
            ),
            Err(CounterexampleSearchError::ObservableSetMismatch)
        );

        let invalid = CounterexampleSearchConfigV1 {
            absolute_tolerance: f64::NAN,
            ..config()
        };
        let same = ValidatedModelProgramV0::new(affine_program(false, 9)).unwrap();
        assert_eq!(
            find_lexicographically_first_counterexample(
                &left,
                &same,
                &[binding_x(1.0)],
                invalid
            ),
            Err(CounterexampleSearchError::InvalidTolerance)
        );
    }
}
