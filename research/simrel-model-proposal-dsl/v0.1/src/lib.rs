//! Closed, typed, deterministic model-proposal AST and interpreter.
//!
//! This crate executes only the expression variants defined below. It never
//! compiles or evaluates arbitrary source text, shell commands, native code or
//! dynamically loaded plugins. The first schema supports static algebraic maps,
//! synchronous discrete state transitions, and seeded stochastic transitions.
//!
//! All updates in a step read the same pre-step snapshot and commit together.
//! Stochastic draws are keyed by (seed, step, channel ID), so AST ordering does
//! not change the random coordinate. The 64-bit fingerprint is reproducible,
//! but deliberately NOT cryptographic.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::Write as _;

pub const SCHEMA_VERSION_V0: u16 = 1;
pub const MAX_VARIABLES: usize = 256;
pub const MAX_ASSIGNMENTS: usize = 256;
pub const MAX_EXPRESSION_NODES: usize = 10_000;
pub const MAX_EXPRESSION_DEPTH: usize = 64;
pub const MAX_HORIZON_STEPS: u32 = 1_000_000;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MIX_INCREMENT: u64 = 0x9e3779b97f4a7c15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VariableId(pub u16);

/// Exponents over v0 base dimensions: length, mass, time and temperature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimension {
    pub exponents: [i8; 4],
}

impl Dimension {
    pub const DIMENSIONLESS: Self = Self {
        exponents: [0, 0, 0, 0],
    };
    pub const LENGTH: Self = Self {
        exponents: [1, 0, 0, 0],
    };
    pub const MASS: Self = Self {
        exponents: [0, 1, 0, 0],
    };
    pub const TIME: Self = Self {
        exponents: [0, 0, 1, 0],
    };
    pub const TEMPERATURE: Self = Self {
        exponents: [0, 0, 0, 1],
    };

    fn multiply(self, other: Self) -> Result<Self, ModelError> {
        let mut exponents = [0_i8; 4];
        for (index, (left, right)) in self
            .exponents
            .iter()
            .zip(other.exponents.iter())
            .enumerate()
        {
            exponents[index] = left
                .checked_add(*right)
                .ok_or(ModelError::DimensionExponentOverflow)?;
        }
        Ok(Self { exponents })
    }

    fn divide(self, other: Self) -> Result<Self, ModelError> {
        let mut exponents = [0_i8; 4];
        for (index, (left, right)) in self
            .exponents
            .iter()
            .zip(other.exponents.iter())
            .enumerate()
        {
            exponents[index] = left
                .checked_sub(*right)
                .ok_or(ModelError::DimensionExponentOverflow)?;
        }
        Ok(Self { exponents })
    }

    pub fn is_dimensionless(self) -> bool {
        self == Self::DIMENSIONLESS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableRole {
    State,
    Latent,
    Input,
    Parameter,
    Observable,
}

impl VariableRole {
    fn as_str(self) -> &'static str {
        match self {
            Self::State => "state",
            Self::Latent => "latent",
            Self::Input => "input",
            Self::Parameter => "parameter",
            Self::Observable => "observable",
        }
    }

    fn is_dynamic_state(self) -> bool {
        matches!(self, Self::State | Self::Latent)
    }

    fn requires_external_binding(self) -> bool {
        matches!(self, Self::Input | Self::Parameter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueRange {
    pub minimum: f64,
    pub maximum: f64,
}

impl ValueRange {
    pub fn validate(self) -> Result<(), ModelError> {
        if !self.minimum.is_finite()
            || !self.maximum.is_finite()
            || self.minimum > self.maximum
        {
            return Err(ModelError::InvalidValueRange);
        }
        Ok(())
    }

    fn contains(self, value: f64) -> bool {
        value >= self.minimum && value <= self.maximum
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    pub id: VariableId,
    pub name: String,
    pub role: VariableRole,
    pub dimension: Dimension,
    pub admissible_range: Option<ValueRange>,
    /// Required only for State/Latent; forbidden for other roles.
    pub initial_value: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Constant {
        value: f64,
        dimension: Dimension,
    },
    Variable(VariableId),
    Add(Box<Expr>, Box<Expr>),
    Subtract(Box<Expr>, Box<Expr>),
    Multiply(Box<Expr>, Box<Expr>),
    Divide(Box<Expr>, Box<Expr>),
    Negate(Box<Expr>),
    Exponential(Box<Expr>),
    /// Unit-normal draw for a stable stochastic coordinate.
    /// Scale it with a dimensioned expression before adding it to a state.
    StandardNormal {
        channel_id: u32,
    },
}

impl Expr {
    pub fn constant(value: f64, dimension: Dimension) -> Self {
        Self::Constant { value, dimension }
    }

    pub fn variable(id: VariableId) -> Self {
        Self::Variable(id)
    }

    pub fn add(left: Self, right: Self) -> Self {
        Self::Add(Box::new(left), Box::new(right))
    }

    pub fn subtract(left: Self, right: Self) -> Self {
        Self::Subtract(Box::new(left), Box::new(right))
    }

    pub fn multiply(left: Self, right: Self) -> Self {
        Self::Multiply(Box::new(left), Box::new(right))
    }

    pub fn divide(left: Self, right: Self) -> Self {
        Self::Divide(Box::new(left), Box::new(right))
    }

    pub fn negate(value: Self) -> Self {
        Self::Negate(Box::new(value))
    }

    pub fn exp(value: Self) -> Self {
        Self::Exponential(Box::new(value))
    }

    fn validate(
        &self,
        symbols: &BTreeMap<VariableId, VariableDecl>,
        stochastic: bool,
        depth: usize,
        node_count: &mut usize,
    ) -> Result<Dimension, ModelError> {
        *node_count = node_count
            .checked_add(1)
            .ok_or(ModelError::ExpressionTooLarge)?;
        if *node_count > MAX_EXPRESSION_NODES {
            return Err(ModelError::ExpressionTooLarge);
        }
        if depth > MAX_EXPRESSION_DEPTH {
            return Err(ModelError::ExpressionTooDeep);
        }

        match self {
            Self::Constant { value, dimension } => {
                if !value.is_finite() {
                    return Err(ModelError::NonFiniteInitialState(declaration.id));
                }
                Ok(*dimension)
            }
            Self::Variable(id) => {
                let declaration = symbols
                    .get(id)
                    .ok_or(ModelError::UnknownVariable(*id))?;
                if declaration.role == VariableRole::Observable {
                    return Err(ModelError::ObservableCannotBeRead(*id));
                }
                Ok(declaration.dimension)
            }
            Self::Add(left, right) | Self::Subtract(left, right) => {
                let left_dimension =
                    left.validate(symbols, stochastic, depth + 1, node_count)?;
                let right_dimension =
                    right.validate(symbols, stochastic, depth + 1, node_count)?;
                if left_dimension != right_dimension {
                    return Err(ModelError::DimensionMismatch {
                        left: left_dimension,
                        right: right_dimension,
                    });
                }
                Ok(left_dimension)
            }
            Self::Multiply(left, right) => {
                let left_dimension =
                    left.validate(symbols, stochastic, depth + 1, node_count)?;
                let right_dimension =
                    right.validate(symbols, stochastic, depth + 1, node_count)?;
                left_dimension.multiply(right_dimension)
            }
            Self::Divide(left, right) => {
                let left_dimension =
                    left.validate(symbols, stochastic, depth + 1, node_count)?;
                let right_dimension =
                    right.validate(symbols, stochastic, depth + 1, node_count)?;
                left_dimension.divide(right_dimension)
            }
            Self::Negate(value) => {
                value.validate(symbols, stochastic, depth + 1, node_count)
            }
            Self::Exponential(value) => {
                let dimension = value.validate(symbols, stochastic, depth + 1, node_count)?;
                if !dimension.is_dimensionless() {
                    return Err(ModelError::DimensionlessArgumentRequired);
                }
                Ok(Dimension::DIMENSIONLESS)
            }
            Self::StandardNormal { .. } => {
                if !stochastic {
                    return Err(ModelError::NoiseOutsideStochasticModel);
                }
                Ok(Dimension::DIMENSIONLESS)
            }
        }
    }

    fn count_noise_nodes(&self) -> usize {
        match self {
            Self::StandardNormal { .. } => 1,
            Self::Add(left, right)
            | Self::Subtract(left, right)
            | Self::Multiply(left, right)
            | Self::Divide(left, right) => {
                left.count_noise_nodes() + right.count_noise_nodes()
            }
            Self::Negate(value) | Self::Exponential(value) => value.count_noise_nodes(),
            Self::Constant { .. } | Self::Variable(_) => 0,
        }
    }

    fn canonical(&self, output: &mut String) {
        match self {
            Self::Constant { value, dimension } => {
                let _ = write!(
                    output,
                    "const:{:016x}:{};",
                    value.to_bits(),
                    dimension_text(*dimension)
                );
            }
            Self::Variable(id) => {
                let _ = write!(output, "var:{};", id.0);
            }
            Self::Add(left, right) => {
                output.push_str("add(");
                left.canonical(output);
                right.canonical(output);
                output.push_str(");");
            }
            Self::Subtract(left, right) => {
                output.push_str("sub(");
                left.canonical(output);
                right.canonical(output);
                output.push_str(");");
            }
            Self::Multiply(left, right) => {
                output.push_str("mul(");
                left.canonical(output);
                right.canonical(output);
                output.push_str(");");
            }
            Self::Divide(left, right) => {
                output.push_str("div(");
                left.canonical(output);
                right.canonical(output);
                output.push_str(");");
            }
            Self::Negate(value) => {
                output.push_str("neg(");
                value.canonical(output);
                output.push_str(");");
            }
            Self::Exponential(value) => {
                output.push_str("exp(");
                value.canonical(output);
                output.push_str(");");
            }
            Self::StandardNormal { channel_id } => {
                let _ = write!(output, "normal:{channel_id};");
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposalClass {
    Algebraic,
    DiscreteState,
    StochasticState,
}

impl ProposalClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Algebraic => "algebraic",
            Self::DiscreteState => "discrete_state",
            Self::StochasticState => "stochastic_state",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assignment {
    pub target: VariableId,
    pub expression: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelProgramV0 {
    pub schema_version: u16,
    pub model_id: String,
    pub variables: Vec<VariableDecl>,
    pub class: ProposalClass,
    pub transitions: Vec<Assignment>,
    pub outputs: Vec<Assignment>,
    pub maximum_horizon_steps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    EmptyModelId,
    InvalidModelId,
    TooManyVariables,
    TooManyAssignments,
    DuplicateVariableId(VariableId),
    DuplicateVariableName,
    InvalidVariableName,
    InvalidValueRange,
    MissingVariableRange(VariableId),
    MissingInitialValue(VariableId),
    UnexpectedInitialValue(VariableId),
    InitialValueOutsideRange(VariableId),
    MissingStateTransitions,
    AlgebraicModelHasTransitions,
    TransitionForNonState(VariableId),
    DuplicateAssignment(VariableId),
    MissingAssignment(VariableId),
    OutputForNonObservable(VariableId),
    NoObservables,
    NoStateVariables,
    AlgebraicModelHasState,
    StatefulModelHasNoState,
    UnknownVariable(VariableId),
    ObservableCannotBeRead(VariableId),
    DimensionMismatch { left: Dimension, right: Dimension },
    DimensionExponentOverflow,
    DimensionlessArgumentRequired,
    NoiseOutsideStochasticModel,
    StochasticModelWithoutNoise,
    NonFiniteConstant,
    NonFiniteInitialState(VariableId),
    NonFiniteBinding(VariableId),
    MissingBinding(VariableId),
    DuplicateBinding(VariableId),
    BindingForNonExternalVariable(VariableId),
    BindingOutsideRange(VariableId),
    InvalidHorizon,
    FuelExhausted,
    ExpressionTooLarge,
    ExpressionTooDeep,
    DivisionByZero,
    NonFiniteEvaluation,
    StateOutsideRange(VariableId),
    InternalMissingValue(VariableId),
    InvalidBudget,
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyModelId => write!(f, "model identity must not be empty"),
            Self::InvalidModelId => write!(f, "model identity must use the admitted ASCII token syntax"),
            Self::TooManyVariables => write!(f, "model proposal exceeds the variable limit"),
            Self::TooManyAssignments => write!(f, "model proposal exceeds the assignment limit"),
            Self::DuplicateVariableId(id) => write!(f, "duplicate variable ID {}", id.0),
            Self::DuplicateVariableName => write!(f, "duplicate variable name"),
            Self::InvalidVariableName => write!(f, "variable names must be lowercase ASCII identifiers"),
            Self::InvalidValueRange => write!(f, "value range must have finite ordered endpoints"),
            Self::MissingVariableRange(id) => write!(f, "input, parameter and dynamic-state variable {} requires an explicit admissible range", id.0),
            Self::MissingInitialValue(id) => write!(f, "state variable {} has no initial value", id.0),
            Self::UnexpectedInitialValue(id) => write!(f, "non-state variable {} declares an initial value", id.0),
            Self::InitialValueOutsideRange(id) => write!(f, "initial state {} is outside its admitted range", id.0),
            Self::MissingStateTransitions => write!(f, "every state/latent variable needs exactly one transition"),
            Self::AlgebraicModelHasTransitions => write!(f, "algebraic proposal cannot declare state transitions"),
            Self::TransitionForNonState(id) => write!(f, "transition target {} is not a state/latent variable", id.0),
            Self::DuplicateAssignment(id) => write!(f, "duplicate assignment for variable {}", id.0),
            Self::MissingAssignment(id) => write!(f, "missing required assignment for variable {}", id.0),
            Self::OutputForNonObservable(id) => write!(f, "output target {} is not observable", id.0),
            Self::NoObservables => write!(f, "proposal must declare at least one observable"),
            Self::NoStateVariables => write!(f, "stateful proposal must declare at least one state variable"),
            Self::AlgebraicModelHasState => write!(f, "algebraic proposal cannot declare state/latent variables"),
            Self::StatefulModelHasNoState => write!(f, "discrete proposal must declare state/latent variables"),
            Self::UnknownVariable(id) => write!(f, "expression references unknown variable {}", id.0),
            Self::ObservableCannotBeRead(id) => write!(f, "observable {} cannot be read by the v0 transition language", id.0),
            Self::DimensionMismatch { .. } => write!(f, "addition/subtraction operands have incompatible dimensions"),
            Self::DimensionExponentOverflow => write!(f, "unit dimension exponent overflows the admitted representation"),
            Self::DimensionlessArgumentRequired => write!(f, "exponential expects a dimensionless argument"),
            Self::NoiseOutsideStochasticModel => write!(f, "random innovations are admitted only in stochastic-state proposals"),
            Self::StochasticModelWithoutNoise => write!(f, "stochastic-state proposal must contain at least one noise node"),
            Self::NonFiniteConstant => write!(f, "expression constant must be finite"),
            Self::NonFiniteInitialState(id) => write!(f, "initial state {} must be finite", id.0),
            Self::NonFiniteBinding(id) => write!(f, "binding for variable {} must be finite", id.0),
            Self::MissingBinding(id) => write!(f, "external binding missing for variable {}", id.0),
            Self::DuplicateBinding(id) => write!(f, "duplicate external binding for variable {}", id.0),
            Self::BindingForNonExternalVariable(id) => write!(f, "binding supplied for non-input/non-parameter variable {}", id.0),
            Self::BindingOutsideRange(id) => write!(f, "external binding for variable {} is outside its admitted range", id.0),
            Self::InvalidHorizon => write!(f, "requested horizon is outside the program's declared limit"),
            Self::FuelExhausted => write!(f, "model interpreter exhausted its expression fuel"),
            Self::ExpressionTooLarge => write!(f, "expression tree exceeds the node limit"),
            Self::ExpressionTooDeep => write!(f, "expression tree exceeds the depth limit"),
            Self::DivisionByZero => write!(f, "model expression divided by zero"),
            Self::NonFiniteEvaluation => write!(f, "model expression produced a non-finite value"),
            Self::StateOutsideRange(id) => write!(f, "updated state {} is outside its admitted range", id.0),
            Self::InternalMissingValue(id) => write!(f, "interpreter environment lacks variable {}", id.0),
            Self::InvalidBudget => write!(f, "interpreter budget must have positive step and expression limits"),
        }
    }
}

impl std::error::Error for ModelError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaluationBudget {
    pub maximum_steps: u32,
    pub expression_fuel: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalBinding {
    pub variable: VariableId,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservableValue {
    pub variable: VariableId,
    pub name: String,
    pub dimension: Dimension,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateValue {
    pub variable: VariableId,
    pub name: String,
    pub dimension: Dimension,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepOutput {
    /// Outputs observe the pre-transition state at this step.
    pub step: u32,
    pub values: Vec<ObservableValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelRunV0 {
    pub model_id: String,
    pub model_fingerprint64: u64,
    pub run_fingerprint64: u64,
    pub fingerprint_algorithm: &'static str,
    pub seed: u64,
    pub requested_steps: u32,
    pub budget: EvaluationBudget,
    pub samples: Vec<StepOutput>,
    pub final_state: Vec<StateValue>,
    pub expressions_evaluated: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedModelProgramV0 {
    program: ModelProgramV0,
    canonical_serialization: String,
    fingerprint64: u64,
}

impl ValidatedModelProgramV0 {
    pub fn new(program: ModelProgramV0) -> Result<Self, ModelError> {
        validate_program(&program)?;
        let canonical_serialization = canonical_serialization(&program);
        let fingerprint64 = fnv1a64(canonical_serialization.as_bytes());
        Ok(Self {
            program,
            canonical_serialization,
            fingerprint64,
        })
    }

    pub fn program(&self) -> &ModelProgramV0 {
        &self.program
    }

    pub fn canonical_serialization(&self) -> &str {
        &self.canonical_serialization
    }

    pub fn fingerprint64(&self) -> u64 {
        self.fingerprint64
    }

    pub fn simulate(
        &self,
        bindings: &[ExternalBinding],
        steps: u32,
        seed: u64,
        budget: EvaluationBudget,
    ) -> Result<ModelRunV0, ModelError> {
        if budget.maximum_steps == 0 || budget.expression_fuel == 0 {
            return Err(ModelError::InvalidBudget);
        }
        if steps == 0
            || steps > self.program.maximum_horizon_steps
            || steps > budget.maximum_steps
            || (self.program.class == ProposalClass::Algebraic && steps != 1)
        {
            return Err(ModelError::InvalidHorizon);
        }

        let symbols: BTreeMap<VariableId, VariableDecl> = self
            .program
            .variables
            .iter()
            .cloned()
            .map(|declaration| (declaration.id, declaration))
            .collect();
        let mut environment = BTreeMap::new();
        let mut seen_bindings = BTreeSet::new();
        for binding in bindings {
            if !seen_bindings.insert(binding.variable) {
                return Err(ModelError::DuplicateBinding(binding.variable));
            }
            let declaration = symbols
                .get(&binding.variable)
                .ok_or(ModelError::UnknownVariable(binding.variable))?;
            if !declaration.role.requires_external_binding() {
                return Err(ModelError::BindingForNonExternalVariable(binding.variable));
            }
            if !binding.value.is_finite() {
                return Err(ModelError::NonFiniteBinding(binding.variable));
            }
            if declaration
                .admissible_range
                .is_some_and(|range| !range.contains(binding.value))
            {
                return Err(ModelError::BindingOutsideRange(binding.variable));
            }
            environment.insert(binding.variable, binding.value);
        }
        for declaration in &self.program.variables {
            if declaration.role.requires_external_binding()
                && !seen_bindings.contains(&declaration.id)
            {
                return Err(ModelError::MissingBinding(declaration.id));
            }
            if declaration.role.is_dynamic_state() {
                environment.insert(
                    declaration.id,
                    declaration
                        .initial_value
                        .ok_or(ModelError::MissingInitialValue(declaration.id))?,
                );
            }
        }

        let mut fuel = budget.expression_fuel;
        let mut expressions_evaluated = 0_u64;
        let mut samples = Vec::with_capacity(steps as usize);
        for step in 0..steps {
            let mut observed = Vec::with_capacity(self.program.outputs.len());
            for assignment in &self.program.outputs {
                let value = evaluate_expression(
                    &assignment.expression,
                    &environment,
                    seed,
                    step,
                    0,
                    &mut fuel,
                    &mut expressions_evaluated,
                )?;
                let declaration = symbols
                    .get(&assignment.target)
                    .ok_or(ModelError::UnknownVariable(assignment.target))?;
                observed.push(ObservableValue {
                    variable: assignment.target,
                    name: declaration.name.clone(),
                    dimension: declaration.dimension,
                    value,
                });
            }
            observed.sort_by_key(|value| value.variable);
            samples.push(StepOutput {
                step,
                values: observed,
            });

            if self.program.class != ProposalClass::Algebraic {
                let mut next_values = Vec::with_capacity(self.program.transitions.len());
                for assignment in &self.program.transitions {
                    let value = evaluate_expression(
                        &assignment.expression,
                        &environment,
                        seed,
                        step,
                        0,
                        &mut fuel,
                        &mut expressions_evaluated,
                    )?;
                    let declaration = symbols
                        .get(&assignment.target)
                        .ok_or(ModelError::UnknownVariable(assignment.target))?;
                    if declaration
                        .admissible_range
                        .is_some_and(|range| !range.contains(value))
                    {
                        return Err(ModelError::StateOutsideRange(assignment.target));
                    }
                    next_values.push((assignment.target, value));
                }
                for (variable, value) in next_values {
                    environment.insert(variable, value);
                }
            }
        }

        let mut final_state = Vec::new();
        for declaration in &self.program.variables {
            if declaration.role.is_dynamic_state() {
                let value = environment
                    .get(&declaration.id)
                    .copied()
                    .ok_or(ModelError::InternalMissingValue(declaration.id))?;
                final_state.push(StateValue {
                    variable: declaration.id,
                    name: declaration.name.clone(),
                    dimension: declaration.dimension,
                    value,
                });
            }
        }
        final_state.sort_by_key(|value| value.variable);

        let run_fingerprint64 = run_fingerprint(
            self.fingerprint64,
            bindings,
            steps,
            seed,
            budget,
        );
        Ok(ModelRunV0 {
            model_id: self.program.model_id.clone(),
            model_fingerprint64: self.fingerprint64,
            run_fingerprint64,
            fingerprint_algorithm: "FNV-1a-64 (reproducibility only; not cryptographic)",
            seed,
            requested_steps: steps,
            budget,
            samples,
            final_state,
            expressions_evaluated,
        })
    }
}

fn validate_program(program: &ModelProgramV0) -> Result<(), ModelError> {
    if program.schema_version != SCHEMA_VERSION_V0 {
        return Err(ModelError::WrongSchemaVersion);
    }
    if program.model_id.trim().is_empty() {
        return Err(ModelError::EmptyModelId);
    }
    if !is_valid_token(&program.model_id) {
        return Err(ModelError::InvalidModelId);
    }
    if program.variables.is_empty() || program.variables.len() > MAX_VARIABLES {
        return Err(ModelError::TooManyVariables);
    }
    if program.outputs.len() > MAX_ASSIGNMENTS
        || program.transitions.len() > MAX_ASSIGNMENTS
    {
        return Err(ModelError::TooManyAssignments);
    }
    if program.maximum_horizon_steps == 0
        || program.maximum_horizon_steps > MAX_HORIZON_STEPS
    {
        return Err(ModelError::InvalidHorizon);
    }

    let mut symbols = BTreeMap::new();
    let mut names = HashSet::new();
    for declaration in &program.variables {
        if !is_valid_token(&declaration.name) {
            return Err(ModelError::InvalidVariableName);
        }
        if !names.insert(declaration.name.clone()) {
            return Err(ModelError::DuplicateVariableName);
        }
        if symbols.insert(declaration.id, declaration.clone()).is_some() {
            return Err(ModelError::DuplicateVariableId(declaration.id));
        }
        if let Some(range) = declaration.admissible_range {
            range.validate()?;
        } else if declaration.role.is_dynamic_state() || declaration.role.requires_external_binding() {
            return Err(ModelError::MissingVariableRange(declaration.id));
        }
        if declaration.role.is_dynamic_state() {
            let initial = declaration
                .initial_value
                .ok_or(ModelError::MissingInitialValue(declaration.id))?;
            if !initial.is_finite() {
                return Err(ModelError::NonFiniteConstant);
            }
            if declaration
                .admissible_range
                .is_some_and(|range| !range.contains(initial))
            {
                return Err(ModelError::InitialValueOutsideRange(declaration.id));
            }
        } else if declaration.initial_value.is_some() {
            return Err(ModelError::UnexpectedInitialValue(declaration.id));
        }
    }

    let state_ids: BTreeSet<VariableId> = program
        .variables
        .iter()
        .filter(|declaration| declaration.role.is_dynamic_state())
        .map(|declaration| declaration.id)
        .collect();
    let observable_ids: BTreeSet<VariableId> = program
        .variables
        .iter()
        .filter(|declaration| declaration.role == VariableRole::Observable)
        .map(|declaration| declaration.id)
        .collect();

    if observable_ids.is_empty() || program.outputs.is_empty() {
        return Err(ModelError::NoObservables);
    }
    if state_ids.is_empty() && program.class != ProposalClass::Algebraic {
        return Err(ModelError::NoStateVariables);
    }
    if !state_ids.is_empty() && program.class == ProposalClass::Algebraic {
        return Err(ModelError::AlgebraicModelHasState);
    }

    let stochastic = program.class == ProposalClass::StochasticState;
    let mut node_count = 0_usize;
    let mut noise_node_count = 0_usize;
    validate_assignment_set(
        &program.outputs,
        &observable_ids,
        false,
        stochastic,
        &symbols,
        &mut node_count,
        &mut noise_node_count,
        ModelError::OutputForNonObservable,
    )?;

    match program.class {
        ProposalClass::Algebraic => {
            if !program.transitions.is_empty() {
                return Err(ModelError::AlgebraicModelHasTransitions);
            }
        }
        ProposalClass::DiscreteState | ProposalClass::StochasticState => {
            if state_ids.is_empty() {
                return Err(ModelError::StatefulModelHasNoState);
            }
            validate_assignment_set(
                &program.transitions,
                &state_ids,
                true,
                stochastic,
                &symbols,
                &mut node_count,
                &mut noise_node_count,
                ModelError::TransitionForNonState,
            )?;
        }
    }

    if program.class == ProposalClass::StochasticState && noise_node_count == 0 {
        return Err(ModelError::StochasticModelWithoutNoise);
    }
    Ok(())
}

fn validate_assignment_set(
    assignments: &[Assignment],
    required_targets: &BTreeSet<VariableId>,
    require_state_target: bool,
    stochastic: bool,
    symbols: &BTreeMap<VariableId, VariableDecl>,
    node_count: &mut usize,
    noise_node_count: &mut usize,
    wrong_target: fn(VariableId) -> ModelError,
) -> Result<(), ModelError> {
    if assignments.is_empty() {
        return Err(ModelError::MissingAssignment(
            *required_targets.iter().next().unwrap_or(&VariableId(0)),
        ));
    }
    let mut targets = BTreeSet::new();
    for assignment in assignments {
        let declaration = symbols
            .get(&assignment.target)
            .ok_or(ModelError::UnknownVariable(assignment.target))?;
        if !required_targets.contains(&assignment.target) {
            return Err(wrong_target(assignment.target));
        }
        if require_state_target && !declaration.role.is_dynamic_state() {
            return Err(wrong_target(assignment.target));
        }
        if !targets.insert(assignment.target) {
            return Err(ModelError::DuplicateAssignment(assignment.target));
        }
        let dimension =
            assignment
                .expression
                .validate(symbols, stochastic, 0, node_count)?;
        *noise_node_count += assignment.expression.count_noise_nodes();
        if dimension != declaration.dimension {
            return Err(ModelError::DimensionMismatch {
                left: dimension,
                right: declaration.dimension,
            });
        }
    }
    for required in required_targets {
        if !targets.contains(required) {
            return Err(ModelError::MissingAssignment(*required));
        }
    }
    Ok(())
}

fn evaluate_expression(
    expression: &Expr,
    environment: &BTreeMap<VariableId, f64>,
    seed: u64,
    step: u32,
    depth: usize,
    fuel: &mut u64,
    evaluated: &mut u64,
) -> Result<f64, ModelError> {
    if *fuel == 0 {
        return Err(ModelError::FuelExhausted);
    }
    *fuel -= 1;
    *evaluated = (*evaluated).saturating_add(1);
    if depth > MAX_EXPRESSION_DEPTH {
        return Err(ModelError::ExpressionTooDeep);
    }

    let value = match expression {
        Expr::Constant { value, .. } => *value,
        Expr::Variable(id) => environment
            .get(id)
            .copied()
            .ok_or(ModelError::InternalMissingValue(*id))?,
        Expr::Add(left, right) => {
            evaluate_expression(left, environment, seed, step, depth + 1, fuel, evaluated)?
                + evaluate_expression(right, environment, seed, step, depth + 1, fuel, evaluated)?
        }
        Expr::Subtract(left, right) => {
            evaluate_expression(left, environment, seed, step, depth + 1, fuel, evaluated)?
                - evaluate_expression(right, environment, seed, step, depth + 1, fuel, evaluated)?
        }
        Expr::Multiply(left, right) => {
            evaluate_expression(left, environment, seed, step, depth + 1, fuel, evaluated)?
                * evaluate_expression(right, environment, seed, step, depth + 1, fuel, evaluated)?
        }
        Expr::Divide(left, right) => {
            let numerator =
                evaluate_expression(left, environment, seed, step, depth + 1, fuel, evaluated)?;
            let denominator =
                evaluate_expression(right, environment, seed, step, depth + 1, fuel, evaluated)?;
            if denominator == 0.0 {
                return Err(ModelError::DivisionByZero);
            }
            numerator / denominator
        }
        Expr::Negate(inner) => {
            -evaluate_expression(inner, environment, seed, step, depth + 1, fuel, evaluated)?
        }
        Expr::Exponential(inner) => {
            evaluate_expression(inner, environment, seed, step, depth + 1, fuel, evaluated)?.exp()
        }
        Expr::StandardNormal { channel_id } => {
            deterministic_normal(seed, step, *channel_id)
        }
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ModelError::NonFiniteEvaluation)
    }
}

fn deterministic_normal(seed: u64, step: u32, channel_id: u32) -> f64 {
    let coordinate = seed
        ^ (u64::from(step).wrapping_mul(0xd6e8feb86659fd93))
        ^ (u64::from(channel_id).wrapping_mul(0xa0761d6478bd642f));
    let mut state = coordinate;
    let u1 = uniform_open(&mut state);
    let u2 = uniform_open(&mut state);
    (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
}

fn uniform_open(state: &mut u64) -> f64 {
    *state = state.wrapping_add(MIX_INCREMENT);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^= value >> 31;
    let mantissa = value >> 11;
    (mantissa as f64 + 0.5) / 9_007_199_254_740_992.0
}

fn canonical_serialization(program: &ModelProgramV0) -> String {
    let mut output = String::new();
    let _ = writeln!(
        output,
        "simrel-model-program-v{}|{}|{}|{}",
        program.schema_version,
        program.model_id,
        program.class.as_str(),
        program.maximum_horizon_steps
    );
    let mut variables = program.variables.clone();
    variables.sort_by_key(|declaration| declaration.id);
    for variable in variables {
        let _ = write!(
            output,
            "variable|{}|{}|{}|{}|",
            variable.id.0,
            variable.name,
            variable.role.as_str(),
            dimension_text(variable.dimension)
        );
        match variable.admissible_range {
            Some(range) => {
                let _ = write!(
                    output,
                    "range:{:016x}:{:016x}|",
                    range.minimum.to_bits(),
                    range.maximum.to_bits()
                );
            }
            None => output.push_str("range:none|"),
        }
        match variable.initial_value {
            Some(value) => {
                let _ = write!(output, "initial:{:016x}\n", value.to_bits());
            }
            None => output.push_str("initial:none\n"),
        }
    }
    let mut transitions = program.transitions.clone();
    transitions.sort_by_key(|assignment| assignment.target);
    for assignment in transitions {
        let _ = write!(output, "transition|{}|", assignment.target.0);
        assignment.expression.canonical(&mut output);
        output.push('\n');
    }
    let mut outputs = program.outputs.clone();
    outputs.sort_by_key(|assignment| assignment.target);
    for assignment in outputs {
        let _ = write!(output, "output|{}|", assignment.target.0);
        assignment.expression.canonical(&mut output);
        output.push('\n');
    }
    output
}

fn run_fingerprint(
    model_fingerprint: u64,
    bindings: &[ExternalBinding],
    steps: u32,
    seed: u64,
    budget: EvaluationBudget,
) -> u64 {
    let mut ordered = bindings.to_vec();
    ordered.sort_by_key(|binding| binding.variable);
    let mut canonical = format!(
        "model:{model_fingerprint:016x}|steps:{steps}|seed:{seed:016x}|max_steps:{}|fuel:{}|",
        budget.maximum_steps,
        budget.expression_fuel
    );
    for binding in ordered {
        let _ = write!(
            canonical,
            "binding:{}:{:016x}|",
            binding.variable.0,
            binding.value.to_bits()
        );
    }
    fnv1a64(canonical.as_bytes())
}

fn dimension_text(dimension: Dimension) -> String {
    format!(
        "{},{},{},{}",
        dimension.exponents[0],
        dimension.exponents[1],
        dimension.exponents[2],
        dimension.exponents[3]
    )
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn is_valid_token(token: &str) -> bool {
    !token.is_empty()
        && token.len() <= 64
        && token.as_bytes()[0].is_ascii_lowercase()
        && token.as_bytes().iter().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_' || *byte == b'-'
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declaration(
        id: u16,
        name: &str,
        role: VariableRole,
        dimension: Dimension,
        range: Option<ValueRange>,
        initial: Option<f64>,
    ) -> VariableDecl {
        VariableDecl {
            id: VariableId(id),
            name: name.to_owned(),
            role,
            dimension,
            admissible_range: range,
            initial_value: initial,
        }
    }

    fn bound(variable: u16, value: f64) -> ExternalBinding {
        ExternalBinding {
            variable: VariableId(variable),
            value,
        }
    }

    fn budget() -> EvaluationBudget {
        EvaluationBudget {
            maximum_steps: 100,
            expression_fuel: 10_000,
        }
    }

    fn algebraic_program() -> ModelProgramV0 {
        ModelProgramV0 {
            schema_version: SCHEMA_VERSION_V0,
            model_id: "affine_sensor_map".to_owned(),
            variables: vec![
                declaration(
                    1,
                    "x",
                    VariableRole::Input,
                    Dimension::LENGTH,
                    Some(ValueRange { minimum: 0.0, maximum: 10.0 }),
                    None,
                ),
                declaration(
                    2,
                    "gain",
                    VariableRole::Parameter,
                    Dimension::DIMENSIONLESS,
                    Some(ValueRange { minimum: 0.0, maximum: 3.0 }),
                    None,
                ),
                declaration(
                    3,
                    "offset",
                    VariableRole::Parameter,
                    Dimension::LENGTH,
                    Some(ValueRange { minimum: -2.0, maximum: 2.0 }),
                    None,
                ),
                declaration(
                    4,
                    "y",
                    VariableRole::Observable,
                    Dimension::LENGTH,
                    None,
                    None,
                ),
            ],
            class: ProposalClass::Algebraic,
            transitions: Vec::new(),
            outputs: vec![Assignment {
                target: VariableId(4),
                expression: Expr::add(
                    Expr::multiply(
                        Expr::variable(VariableId(1)),
                        Expr::variable(VariableId(2)),
                    ),
                    Expr::variable(VariableId(3)),
                ),
            }],
            maximum_horizon_steps: 1,
        }
    }

    fn discrete_program() -> ModelProgramV0 {
        ModelProgramV0 {
            schema_version: SCHEMA_VERSION_V0,
            model_id: "first_order_dynamics".to_owned(),
            variables: vec![
                declaration(
                    1,
                    "x",
                    VariableRole::State,
                    Dimension::LENGTH,
                    Some(ValueRange { minimum: -100.0, maximum: 100.0 }),
                    Some(1.0),
                ),
                declaration(
                    2,
                    "u",
                    VariableRole::Input,
                    Dimension::LENGTH,
                    Some(ValueRange { minimum: -10.0, maximum: 10.0 }),
                    None,
                ),
                declaration(
                    3,
                    "decay",
                    VariableRole::Parameter,
                    Dimension::DIMENSIONLESS,
                    Some(ValueRange { minimum: 0.0, maximum: 1.0 }),
                    None,
                ),
                declaration(
                    4,
                    "observed_x",
                    VariableRole::Observable,
                    Dimension::LENGTH,
                    None,
                    None,
                ),
            ],
            class: ProposalClass::DiscreteState,
            transitions: vec![Assignment {
                target: VariableId(1),
                expression: Expr::add(
                    Expr::multiply(
                        Expr::variable(VariableId(1)),
                        Expr::variable(VariableId(3)),
                    ),
                    Expr::variable(VariableId(2)),
                ),
            }],
            outputs: vec![Assignment {
                target: VariableId(4),
                expression: Expr::variable(VariableId(1)),
            }],
            maximum_horizon_steps: 100,
        }
    }

    fn stochastic_program() -> ModelProgramV0 {
        ModelProgramV0 {
            schema_version: SCHEMA_VERSION_V0,
            model_id: "random_walk".to_owned(),
            variables: vec![
                declaration(
                    1,
                    "state",
                    VariableRole::Latent,
                    Dimension::DIMENSIONLESS,
                    Some(ValueRange { minimum: -100.0, maximum: 100.0 }),
                    Some(0.0),
                ),
                declaration(
                    2,
                    "sigma",
                    VariableRole::Parameter,
                    Dimension::DIMENSIONLESS,
                    Some(ValueRange { minimum: 0.0, maximum: 1.0 }),
                    None,
                ),
                declaration(
                    3,
                    "signal",
                    VariableRole::Observable,
                    Dimension::DIMENSIONLESS,
                    None,
                    None,
                ),
            ],
            class: ProposalClass::StochasticState,
            transitions: vec![Assignment {
                target: VariableId(1),
                expression: Expr::add(
                    Expr::variable(VariableId(1)),
                    Expr::multiply(
                        Expr::variable(VariableId(2)),
                        Expr::StandardNormal { channel_id: 7 },
                    ),
                ),
            }],
            outputs: vec![Assignment {
                target: VariableId(3),
                expression: Expr::variable(VariableId(1)),
            }],
            maximum_horizon_steps: 100,
        }
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let mut program = algebraic_program();
        program.schema_version = SCHEMA_VERSION_V0 + 1;
        assert_eq!(
            ValidatedModelProgramV0::new(program),
            Err(ModelError::WrongSchemaVersion)
        );
    }

    #[test]
    fn algebraic_proposal_executes_with_typed_bindings() {
        let validated = ValidatedModelProgramV0::new(algebraic_program()).unwrap();
        let run = validated
            .simulate(&[bound(1, 3.0), bound(2, 2.0), bound(3, 1.0)], 1, 1, budget())
            .unwrap();
        assert_eq!(run.samples.len(), 1);
        assert_eq!(run.samples[0].values[0].value, 7.0);
        let reordered = validated
            .simulate(&[bound(3, 1.0), bound(1, 3.0), bound(2, 2.0)], 1, 1, budget())
            .unwrap();
        assert_eq!(run.run_fingerprint64, reordered.run_fingerprint64);
    }

    #[test]
    fn discrete_transitions_are_simultaneous_and_reproducible() {
        let validated = ValidatedModelProgramV0::new(discrete_program()).unwrap();
        let run = validated
            .simulate(&[bound(2, 0.25), bound(3, 0.5)], 3, 1, budget())
            .unwrap();
        let values: Vec<f64> = run
            .samples
            .iter()
            .map(|sample| sample.values[0].value)
            .collect();
        assert_eq!(values, vec![1.0, 0.75, 0.625]);
        assert_eq!(run.final_state[0].value, 0.5625);
    }

    #[test]
    fn stochastic_proposal_is_reproducible_for_a_frozen_seed() {
        let validated = ValidatedModelProgramV0::new(stochastic_program()).unwrap();
        let bindings = [bound(2, 0.25)];
        let first = validated.simulate(&bindings, 10, 1234, budget()).unwrap();
        let second = validated.simulate(&bindings, 10, 1234, budget()).unwrap();
        assert_eq!(first, second);
        let third = validated.simulate(&bindings, 10, 1235, budget()).unwrap();
        assert_ne!(first, third);
    }

    #[test]
    fn ast_order_does_not_change_the_canonical_fingerprint() {
        let original = algebraic_program();
        let fingerprint = ValidatedModelProgramV0::new(original.clone())
            .unwrap()
            .fingerprint64();
        let mut reordered = original;
        reordered.variables.reverse();
        reordered.outputs.reverse();
        let reordered_fingerprint = ValidatedModelProgramV0::new(reordered)
            .unwrap()
            .fingerprint64();
        assert_eq!(fingerprint, reordered_fingerprint);
    }

    #[test]
    fn unit_mismatch_is_rejected_before_execution() {
        let mut program = algebraic_program();
        program.outputs[0].expression = Expr::add(
            Expr::variable(VariableId(1)),
            Expr::variable(VariableId(3)),
        );
        assert!(matches!(
            ValidatedModelProgramV0::new(program),
            Err(ModelError::DimensionMismatch { .. })
        ));
    }

    #[test]
    fn parameter_and_input_bounds_are_enforced() {
        let validated = ValidatedModelProgramV0::new(algebraic_program()).unwrap();
        assert_eq!(
            validated.simulate(
                &[bound(1, 11.0), bound(2, 1.0), bound(3, 0.0)],
                1,
                1,
                budget()
            ),
            Err(ModelError::BindingOutsideRange(VariableId(1)))
        );
    }

    #[test]
    fn duplicate_missing_and_unknown_bindings_fail_closed() {
        let validated = ValidatedModelProgramV0::new(algebraic_program()).unwrap();
        assert_eq!(
            validated.simulate(
                &[bound(1, 1.0), bound(1, 2.0), bound(2, 1.0), bound(3, 0.0)],
                1,
                1,
                budget()
            ),
            Err(ModelError::DuplicateBinding(VariableId(1)))
        );
        assert_eq!(
            validated.simulate(&[bound(1, 1.0), bound(2, 1.0)], 1, 1, budget()),
            Err(ModelError::MissingBinding(VariableId(3)))
        );
        assert_eq!(
            validated.simulate(
                &[bound(1, 1.0), bound(2, 1.0), bound(3, 0.0), bound(99, 1.0)],
                1,
                1,
                budget()
            ),
            Err(ModelError::UnknownVariable(VariableId(99)))
        );
    }

    #[test]
    fn stochastic_noise_is_not_admitted_by_deterministic_proposals() {
        let mut program = discrete_program();
        program.transitions[0].expression = Expr::StandardNormal { channel_id: 9 };
        assert_eq!(
            ValidatedModelProgramV0::new(program),
            Err(ModelError::NoiseOutsideStochasticModel)
        );
    }

    #[test]
    fn algebraic_models_cannot_smuggle_state_and_stateful_models_need_all_updates() {
        let mut algebraic = algebraic_program();
        algebraic.variables.push(declaration(
            5,
            "hidden_state",
            VariableRole::Latent,
            Dimension::DIMENSIONLESS,
            None,
            Some(ValueRange { minimum: -1.0, maximum: 1.0 }),
            Some(0.0),
        ));
        assert_eq!(
            ValidatedModelProgramV0::new(algebraic),
            Err(ModelError::AlgebraicModelHasState)
        );

        let mut discrete = discrete_program();
        discrete.transitions.clear();
        assert_eq!(
            ValidatedModelProgramV0::new(discrete),
            Err(ModelError::MissingAssignment(VariableId(1)))
        );
    }

    #[test]
    fn fuel_and_horizon_limits_are_enforced() {
        let validated = ValidatedModelProgramV0::new(discrete_program()).unwrap();
        assert_eq!(
            validated.simulate(
                &[bound(2, 0.0), bound(3, 0.5)],
                2,
                1,
                EvaluationBudget {
                    maximum_steps: 2,
                    expression_fuel: 1,
                }
            ),
            Err(ModelError::FuelExhausted)
        );
        assert_eq!(
            validated.simulate(&[bound(2, 0.0), bound(3, 0.5)], 101, 1, budget()),
            Err(ModelError::InvalidHorizon)
        );
    }
}
