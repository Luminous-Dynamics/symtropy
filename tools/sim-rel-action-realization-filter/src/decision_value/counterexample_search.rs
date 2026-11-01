//! Deterministic counterexample search over a declared family of kernel mutations.
//!
//! This is a bounded model-falsification primitive, not an open-world proof.
//! Mutations preserve the finite kernel contract and are replayed against one
//! fixed public policy. A new observation branch that the policy cannot handle
//! is itself a concrete counterexample, rather than a branch to be silently
//! marginalized or discarded.

use super::finite_horizon::{DecisionBudgets, DecisionPlan};
use super::robust_finite_horizon::{
    DecisionModelWorld, WorldDecisionObjectives, replay_shared_policy_across_worlds,
};
use super::{DecisionLossTable, ObservationProtocol};
use crate::{ActionRealizationKernel, LatentState, Rational};
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ModelWorldPerturbation {
    /// Swap two measurement symbols for one command and current sensor state.
    SwapMeasurementLabels {
        command: usize,
        sensor_state: usize,
        left_measurement: usize,
        right_measurement: usize,
    },
    /// Invert a sensor's measurement labels across every commanded action.
    /// This models a persistent calibration/sign change, not a one-command fault.
    SwapSensorCalibration {
        sensor_state: usize,
        left_measurement: usize,
        right_measurement: usize,
    },
    /// Split readback-bearing outcomes into delivered and missing-readback paths.
    DropReadback {
        command: usize,
        sensor_state: Option<usize>,
        dropout_probability: Rational,
    },
    /// Split safe outcomes into safe and terminal-hazard paths.
    InjectHazard {
        command: usize,
        sensor_state: Option<usize>,
        hazard_probability: Rational,
    },
    /// Move exact probability mass between two outcomes in one complete row.
    ShiftOutcomeMass {
        state: LatentState,
        command: usize,
        source_outcome: usize,
        target_outcome: usize,
        amount: Rational,
    },
    /// Route subsequent outcomes from a selected apparatus state into a
    /// declared degraded apparatus state.
    DegradeApparatus {
        command: usize,
        current_apparatus_state_at_least: usize,
        degraded_next_state: usize,
    },
    /// Change the hidden realized-action class while preserving the vocabulary.
    RemapRealizedAction {
        command: usize,
        sensor_state: Option<usize>,
        from_action: usize,
        to_action: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PerturbationSpec {
    pub source_world_id: String,
    pub candidate_world_id: String,
    pub perturbation: ModelWorldPerturbation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CounterexampleClass {
    NoViolation,
    DecisionLossAmplified,
    AbstentionBudgetViolated,
    HazardBudgetViolated,
    PolicyInvalidated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CounterexampleOutcome {
    PolicyInvalidated {
        reason: String,
    },
    HazardBudgetViolated {
        metrics: WorldDecisionObjectives,
        allowed_hazard_probability: Rational,
    },
    AbstentionBudgetViolated {
        metrics: WorldDecisionObjectives,
        allowed_abstention_probability: Rational,
    },
    DecisionLossAmplified {
        metrics: WorldDecisionObjectives,
        baseline_worst_case_terminal_loss: Rational,
    },
    NoViolation {
        metrics: WorldDecisionObjectives,
    },
}

impl CounterexampleOutcome {
    pub fn class(&self) -> CounterexampleClass {
        match self {
            Self::PolicyInvalidated { .. } => CounterexampleClass::PolicyInvalidated,
            Self::HazardBudgetViolated { .. } => CounterexampleClass::HazardBudgetViolated,
            Self::AbstentionBudgetViolated { .. } => CounterexampleClass::AbstentionBudgetViolated,
            Self::DecisionLossAmplified { .. } => CounterexampleClass::DecisionLossAmplified,
            Self::NoViolation { .. } => CounterexampleClass::NoViolation,
        }
    }

    fn metrics(&self) -> Option<&WorldDecisionObjectives> {
        match self {
            Self::PolicyInvalidated { .. } => None,
            Self::HazardBudgetViolated { metrics, .. }
            | Self::AbstentionBudgetViolated { metrics, .. }
            | Self::DecisionLossAmplified { metrics, .. }
            | Self::NoViolation { metrics } => Some(metrics),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleWitness {
    pub spec: PerturbationSpec,
    /// The validated, complete model world that falsifies or challenges the policy.
    pub candidate_world: DecisionModelWorld,
    pub outcome: CounterexampleOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RejectedPerturbation {
    pub spec: PerturbationSpec,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterexampleSearchReport {
    /// Exact input snapshots needed to reproduce the falsification.
    pub baseline_worlds: Vec<DecisionModelWorld>,
    pub baseline_world_ids: Vec<String>,
    pub baseline_worst_case_total_loss: Rational,
    pub baseline_worst_case_terminal_loss: Rational,
    pub nominal_world_id: String,
    pub loss_table: DecisionLossTable,
    pub protocol: ObservationProtocol,
    pub command_costs: Vec<Rational>,
    pub hazard_loss: Rational,
    pub policy: DecisionPlan,
    pub horizon: usize,
    pub budgets: DecisionBudgets,
    pub attempted: usize,
    pub witnesses: Vec<CounterexampleWitness>,
    pub non_violating_candidates: Vec<CounterexampleWitness>,
    pub rejected: Vec<RejectedPerturbation>,
}

impl CounterexampleSearchReport {
    pub fn strongest_counterexample(&self) -> Option<&CounterexampleWitness> {
        self.witnesses.first()
    }
}

/// Apply one deterministic, exact perturbation and validate the resulting kernel.
///
/// Every mutation preserves the base world's state/action/observation
/// vocabularies so it can be admitted into the same robust family. The caller
/// supplies a unique candidate ID; it becomes immutable provenance for this
/// generated world.
pub fn apply_model_world_perturbation(
    source: &DecisionModelWorld,
    candidate_world_id: &str,
    perturbation: &ModelWorldPerturbation,
) -> Result<DecisionModelWorld, String> {
    if candidate_world_id.trim().is_empty() || candidate_world_id == source.id {
        return Err("candidate world ID must be non-empty and distinct from its source".into());
    }
    source.kernel.validate()?;
    let mut candidate = source.clone();
    candidate.id = candidate_world_id.to_string();

    match perturbation {
        ModelWorldPerturbation::SwapMeasurementLabels {
            command,
            sensor_state,
            left_measurement,
            right_measurement,
        } => {
            if *command >= candidate.kernel.command_ids.len()
                || *sensor_state >= candidate.kernel.sensor_state_count
                || *left_measurement >= candidate.kernel.measurement_ids.len()
                || *right_measurement >= candidate.kernel.measurement_ids.len()
                || left_measurement == right_measurement
            {
                return Err("measurement-swap parameters are outside the source vocabulary".into());
            }
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if row.command != *command || row.state.sensor_state != *sensor_state {
                    continue;
                }
                for outcome in &mut row.outcomes {
                    match outcome.measurement {
                        Some(value) if value == *left_measurement => {
                            outcome.measurement = Some(*right_measurement);
                            changed = true;
                        }
                        Some(value) if value == *right_measurement => {
                            outcome.measurement = Some(*left_measurement);
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
            require_changed(changed, "measurement swap matched no outcomes")?;
        }
        ModelWorldPerturbation::SwapSensorCalibration {
            sensor_state,
            left_measurement,
            right_measurement,
        } => {
            if *sensor_state >= candidate.kernel.sensor_state_count
                || *left_measurement >= candidate.kernel.measurement_ids.len()
                || *right_measurement >= candidate.kernel.measurement_ids.len()
                || left_measurement == right_measurement
            {
                return Err(
                    "sensor-calibration swap parameters are outside the source vocabulary".into(),
                );
            }
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if row.state.sensor_state != *sensor_state {
                    continue;
                }
                for outcome in &mut row.outcomes {
                    match outcome.measurement {
                        Some(value) if value == *left_measurement => {
                            outcome.measurement = Some(*right_measurement);
                            changed = true;
                        }
                        Some(value) if value == *right_measurement => {
                            outcome.measurement = Some(*left_measurement);
                            changed = true;
                        }
                        _ => {}
                    }
                }
            }
            require_changed(changed, "sensor-calibration swap matched no outcomes")?;
        }
        ModelWorldPerturbation::DropReadback {
            command,
            sensor_state,
            dropout_probability,
        } => {
            validate_split_probability(*dropout_probability, "readback dropout")?;
            validate_scope(&candidate.kernel, *command, *sensor_state)?;
            let keep_probability = complement(*dropout_probability)?;
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if !row_matches(row.state, row.command, *command, *sensor_state) {
                    continue;
                }
                let mut expanded = Vec::with_capacity(row.outcomes.len().saturating_mul(2));
                for outcome in &row.outcomes {
                    if outcome.readback.is_some() {
                        let mut delivered = outcome.clone();
                        delivered.probability =
                            delivered.probability.checked_mul(keep_probability)?;
                        let mut missing = outcome.clone();
                        missing.probability =
                            missing.probability.checked_mul(*dropout_probability)?;
                        missing.readback = None;
                        expanded.push(delivered);
                        expanded.push(missing);
                        changed = true;
                    } else {
                        expanded.push(outcome.clone());
                    }
                }
                row.outcomes = expanded;
            }
            require_changed(
                changed,
                "readback dropout matched no readback-bearing outcomes",
            )?;
        }
        ModelWorldPerturbation::InjectHazard {
            command,
            sensor_state,
            hazard_probability,
        } => {
            validate_split_probability(*hazard_probability, "hazard injection")?;
            validate_scope(&candidate.kernel, *command, *sensor_state)?;
            let safe_probability = complement(*hazard_probability)?;
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if !row_matches(row.state, row.command, *command, *sensor_state) {
                    continue;
                }
                let mut expanded = Vec::with_capacity(row.outcomes.len().saturating_mul(2));
                for outcome in &row.outcomes {
                    if !outcome.hazard {
                        let mut safe = outcome.clone();
                        safe.probability = safe.probability.checked_mul(safe_probability)?;
                        let mut hazard = outcome.clone();
                        hazard.probability = hazard.probability.checked_mul(*hazard_probability)?;
                        hazard.hazard = true;
                        hazard.measurement = None;
                        expanded.push(safe);
                        expanded.push(hazard);
                        changed = true;
                    } else {
                        expanded.push(outcome.clone());
                    }
                }
                row.outcomes = expanded;
            }
            require_changed(changed, "hazard injection matched no safe outcomes")?;
        }
        ModelWorldPerturbation::ShiftOutcomeMass {
            state,
            command,
            source_outcome,
            target_outcome,
            amount,
        } => {
            if *command >= candidate.kernel.command_ids.len()
                || state.hypothesis >= candidate.kernel.hypothesis_ids.len()
                || state.physical_state >= candidate.kernel.physical_state_count
                || state.sensor_state >= candidate.kernel.sensor_state_count
                || state.apparatus_state >= candidate.kernel.apparatus_state_count
                || source_outcome == target_outcome
                || amount.is_zero()
            {
                return Err("outcome-mass shift parameters are invalid".into());
            }
            let row_index = candidate.kernel.row_index(*state, *command);
            let row = &mut candidate.kernel.rows[row_index];
            if *source_outcome >= row.outcomes.len() || *target_outcome >= row.outcomes.len() {
                return Err("outcome-mass shift index is outside the selected row".into());
            }
            let source_probability = row.outcomes[*source_outcome].probability;
            if *amount >= source_probability {
                return Err("outcome-mass shift must leave positive source probability".into());
            }
            row.outcomes[*source_outcome].probability =
                subtract_nonnegative(source_probability, *amount)?;
            row.outcomes[*target_outcome].probability = row.outcomes[*target_outcome]
                .probability
                .checked_add(*amount)?;
        }
        ModelWorldPerturbation::DegradeApparatus {
            command,
            current_apparatus_state_at_least,
            degraded_next_state,
        } => {
            if *command >= candidate.kernel.command_ids.len()
                || *current_apparatus_state_at_least >= candidate.kernel.apparatus_state_count
                || *degraded_next_state >= candidate.kernel.apparatus_state_count
            {
                return Err("apparatus-degradation parameters are outside the state space".into());
            }
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if row.command != *command
                    || row.state.apparatus_state < *current_apparatus_state_at_least
                {
                    continue;
                }
                for outcome in &mut row.outcomes {
                    if outcome.next_state.apparatus_state != *degraded_next_state {
                        outcome.next_state.apparatus_state = *degraded_next_state;
                        changed = true;
                    }
                }
            }
            require_changed(changed, "apparatus degradation matched no next states")?;
        }
        ModelWorldPerturbation::RemapRealizedAction {
            command,
            sensor_state,
            from_action,
            to_action,
        } => {
            validate_scope(&candidate.kernel, *command, *sensor_state)?;
            if *from_action >= candidate.kernel.realized_action_ids.len()
                || *to_action >= candidate.kernel.realized_action_ids.len()
                || from_action == to_action
            {
                return Err("realized-action remap parameters are invalid".into());
            }
            let mut changed = false;
            for row in &mut candidate.kernel.rows {
                if !row_matches(row.state, row.command, *command, *sensor_state) {
                    continue;
                }
                for outcome in &mut row.outcomes {
                    if outcome.realized_action == *from_action {
                        outcome.realized_action = *to_action;
                        changed = true;
                    }
                }
            }
            require_changed(changed, "realized-action remap matched no outcomes")?;
        }
    }

    candidate.kernel.validate()?;
    Ok(candidate)
}

fn require_changed(changed: bool, reason: &str) -> Result<(), String> {
    if changed {
        Ok(())
    } else {
        Err(reason.to_string())
    }
}

fn validate_scope(
    kernel: &ActionRealizationKernel,
    command: usize,
    sensor_state: Option<usize>,
) -> Result<(), String> {
    if command >= kernel.command_ids.len()
        || sensor_state.is_some_and(|state| state >= kernel.sensor_state_count)
    {
        return Err("perturbation command or sensor scope is outside the source model".into());
    }
    Ok(())
}

fn row_matches(
    state: LatentState,
    row_command: usize,
    command: usize,
    sensor_state: Option<usize>,
) -> bool {
    row_command == command && sensor_state.is_none_or(|sensor| state.sensor_state == sensor)
}

fn validate_split_probability(probability: Rational, label: &str) -> Result<(), String> {
    if probability.is_zero() || probability >= Rational::one() {
        return Err(format!(
            "{label} probability must be strictly between zero and one"
        ));
    }
    Ok(())
}

fn complement(probability: Rational) -> Result<Rational, String> {
    if !probability.is_probability() {
        return Err("probability complement requires a value in [0, 1]".into());
    }
    Rational::new(
        probability.denominator() - probability.numerator(),
        probability.denominator(),
    )
}

fn subtract_nonnegative(left: Rational, right: Rational) -> Result<Rational, String> {
    if left < right {
        return Err("rational subtraction would be negative".into());
    }
    let common = gcd(left.denominator(), right.denominator());
    let left_scale = right.denominator() / common;
    let right_scale = left.denominator() / common;
    let left_numerator = left
        .numerator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let right_numerator = right
        .numerator()
        .checked_mul(right_scale)
        .ok_or_else(|| "rational subtraction numerator overflow".to_string())?;
    let denominator = left
        .denominator()
        .checked_mul(left_scale)
        .ok_or_else(|| "rational subtraction denominator overflow".to_string())?;
    Rational::new(left_numerator - right_numerator, denominator)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

/// Parameterized deterministic grid for bounded model-family attacks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedPerturbationGrid {
    /// Probabilities used for dropout and hazard splitting; each must lie in (0, 1).
    pub split_probabilities: Vec<Rational>,
    /// Positive probability mass moved between outcomes in a complete kernel row.
    pub outcome_mass_shifts: Vec<Rational>,
    pub include_measurement_label_swaps: bool,
    pub include_readback_dropout: bool,
    pub include_hazard_injection: bool,
    pub include_outcome_mass_shifts: bool,
    pub include_apparatus_degradation: bool,
    pub include_realized_action_remapping: bool,
    /// Add per-sensor variants in addition to global command/sensor variants.
    pub include_sensor_scoped_variants: bool,
}

impl Default for BoundedPerturbationGrid {
    fn default() -> Self {
        Self {
            split_probabilities: vec![
                Rational::new(1, 20).expect("nonzero denominator"),
                Rational::new(1, 10).expect("nonzero denominator"),
                Rational::new(1, 5).expect("nonzero denominator"),
            ],
            outcome_mass_shifts: vec![
                Rational::new(1, 20).expect("nonzero denominator"),
                Rational::new(1, 10).expect("nonzero denominator"),
            ],
            include_measurement_label_swaps: true,
            include_readback_dropout: true,
            include_hazard_injection: true,
            include_outcome_mass_shifts: true,
            include_apparatus_degradation: true,
            include_realized_action_remapping: true,
            include_sensor_scoped_variants: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedPerturbationGridReport {
    /// All emitted perturbations have already produced a valid candidate kernel.
    pub specs: Vec<PerturbationSpec>,
    /// Deterministic parameter combinations that were inapplicable/no-op.
    pub skipped_inapplicable: usize,
    /// Number of parameter combinations evaluated, including skipped ones.
    pub attempted: usize,
}

/// Enumerate a finite, deterministic family of valid model perturbations.
///
/// Worlds are sorted by ID before enumeration. Candidate IDs and order are
/// therefore invariant to input-world ordering. Every candidate is validated
/// by applying it to its immutable source before it is emitted. The explicit
/// attempt cap bounds Cartesian-product growth; exceeding it returns an error
/// instead of a silently truncated search.
pub fn generate_bounded_perturbation_grid(
    worlds: &[DecisionModelWorld],
    grid: &BoundedPerturbationGrid,
    max_attempts: usize,
) -> Result<BoundedPerturbationGridReport, String> {
    const MAX_GRID_ATTEMPTS: usize = 100_000;
    if worlds.is_empty() {
        return Err("perturbation grid requires at least one source world".into());
    }
    if max_attempts == 0 || max_attempts > MAX_GRID_ATTEMPTS {
        return Err(format!(
            "perturbation grid attempt cap must be in 1..={MAX_GRID_ATTEMPTS}"
        ));
    }

    let mut canonical_worlds = worlds.to_vec();
    canonical_worlds.sort_by(|left, right| left.id.cmp(&right.id));
    let mut world_ids = BTreeSet::new();
    for world in &canonical_worlds {
        if world.id.trim().is_empty() || !world_ids.insert(world.id.as_str()) {
            return Err("source world IDs must be non-empty and unique".into());
        }
        world.kernel.validate()?;
    }

    let mut split_probabilities = grid.split_probabilities.clone();
    split_probabilities.sort();
    split_probabilities.dedup();
    for probability in &split_probabilities {
        validate_split_probability(*probability, "grid split")?;
    }
    let mut mass_shifts = grid.outcome_mass_shifts.clone();
    mass_shifts.sort();
    mass_shifts.dedup();
    if mass_shifts.iter().any(|amount| amount.is_zero()) {
        return Err("grid outcome-mass shifts must be positive".into());
    }

    let mut report = BoundedPerturbationGridReport {
        specs: Vec::new(),
        skipped_inapplicable: 0,
        attempted: 0,
    };

    for source in &canonical_worlds {
        let kernel = &source.kernel;
        if grid.include_measurement_label_swaps {
            for sensor_state in 0..kernel.sensor_state_count {
                for left in 0..kernel.measurement_ids.len() {
                    for right in (left + 1)..kernel.measurement_ids.len() {
                        emit_grid_candidate(
                            &mut report,
                            source,
                            ModelWorldPerturbation::SwapSensorCalibration {
                                sensor_state,
                                left_measurement: left,
                                right_measurement: right,
                            },
                            &world_ids,
                            max_attempts,
                        )?;
                    }
                }
            }
            if grid.include_sensor_scoped_variants {
                for command in 0..kernel.command_ids.len() {
                    for sensor_state in 0..kernel.sensor_state_count {
                        for left in 0..kernel.measurement_ids.len() {
                            for right in (left + 1)..kernel.measurement_ids.len() {
                                emit_grid_candidate(
                                    &mut report,
                                    source,
                                    ModelWorldPerturbation::SwapMeasurementLabels {
                                        command,
                                        sensor_state,
                                        left_measurement: left,
                                        right_measurement: right,
                                    },
                                    &world_ids,
                                    max_attempts,
                                )?;
                            }
                        }
                    }
                }
            }
        }

        for command in 0..kernel.command_ids.len() {
            let mut scopes = vec![None];
            if grid.include_sensor_scoped_variants {
                scopes.extend((0..kernel.sensor_state_count).map(Some));
            }
            for sensor_state in &scopes {
                if grid.include_readback_dropout
                    && kernel.rows.iter().any(|row| {
                        row.command == command
                            && sensor_state.is_none_or(|sensor| row.state.sensor_state == sensor)
                            && row
                                .outcomes
                                .iter()
                                .any(|outcome| outcome.readback.is_some())
                    })
                {
                    for probability in &split_probabilities {
                        emit_grid_candidate(
                            &mut report,
                            source,
                            ModelWorldPerturbation::DropReadback {
                                command,
                                sensor_state: *sensor_state,
                                dropout_probability: *probability,
                            },
                            &world_ids,
                            max_attempts,
                        )?;
                    }
                }
                if grid.include_hazard_injection
                    && kernel.rows.iter().any(|row| {
                        row.command == command
                            && sensor_state.is_none_or(|sensor| row.state.sensor_state == sensor)
                            && row.outcomes.iter().any(|outcome| !outcome.hazard)
                    })
                {
                    for probability in &split_probabilities {
                        emit_grid_candidate(
                            &mut report,
                            source,
                            ModelWorldPerturbation::InjectHazard {
                                command,
                                sensor_state: *sensor_state,
                                hazard_probability: *probability,
                            },
                            &world_ids,
                            max_attempts,
                        )?;
                    }
                }
                if grid.include_realized_action_remapping {
                    for from_action in 0..kernel.realized_action_ids.len() {
                        for to_action in 0..kernel.realized_action_ids.len() {
                            if from_action == to_action {
                                continue;
                            }
                            emit_grid_candidate(
                                &mut report,
                                source,
                                ModelWorldPerturbation::RemapRealizedAction {
                                    command,
                                    sensor_state: *sensor_state,
                                    from_action,
                                    to_action,
                                },
                                &world_ids,
                                max_attempts,
                            )?;
                        }
                    }
                }
            }

            if grid.include_outcome_mass_shifts {
                for row in kernel.rows.iter().filter(|row| row.command == command) {
                    for source_outcome in 0..row.outcomes.len() {
                        for target_outcome in 0..row.outcomes.len() {
                            if source_outcome == target_outcome {
                                continue;
                            }
                            for amount in &mass_shifts {
                                emit_grid_candidate(
                                    &mut report,
                                    source,
                                    ModelWorldPerturbation::ShiftOutcomeMass {
                                        state: row.state,
                                        command,
                                        source_outcome,
                                        target_outcome,
                                        amount: *amount,
                                    },
                                    &world_ids,
                                    max_attempts,
                                )?;
                            }
                        }
                    }
                }
            }

            if grid.include_apparatus_degradation && kernel.apparatus_state_count > 1 {
                for threshold in 0..kernel.apparatus_state_count {
                    for degraded_state in 0..kernel.apparatus_state_count {
                        if threshold == degraded_state {
                            continue;
                        }
                        emit_grid_candidate(
                            &mut report,
                            source,
                            ModelWorldPerturbation::DegradeApparatus {
                                command,
                                current_apparatus_state_at_least: threshold,
                                degraded_next_state: degraded_state,
                            },
                            &world_ids,
                            max_attempts,
                        )?;
                    }
                }
            }
        }
    }

    Ok(report)
}

fn emit_grid_candidate(
    report: &mut BoundedPerturbationGridReport,
    source: &DecisionModelWorld,
    perturbation: ModelWorldPerturbation,
    world_ids: &BTreeSet<&str>,
    max_attempts: usize,
) -> Result<(), String> {
    if report.attempted >= max_attempts {
        return Err("bounded perturbation grid attempt cap exceeded".into());
    }
    report.attempted = report
        .attempted
        .checked_add(1)
        .ok_or_else(|| "perturbation grid attempt counter overflow".to_string())?;
    let ordinal = report
        .specs
        .len()
        .checked_add(1)
        .ok_or_else(|| "candidate ID counter overflow".to_string())?;
    let candidate_world_id = format!("sim-rel-029-generated-{ordinal:08}");
    if world_ids
        .iter()
        .any(|world_id| *world_id == candidate_world_id.as_str())
    {
        return Err("generated candidate ID collides with a source world".into());
    }
    if perturbation_is_applicable(source, &perturbation) {
        report.specs.push(PerturbationSpec {
            source_world_id: source.id.clone(),
            candidate_world_id,
            perturbation,
        });
    } else {
        report.skipped_inapplicable = report
            .skipped_inapplicable
            .checked_add(1)
            .ok_or_else(|| "inapplicable perturbation counter overflow".to_string())?;
    }
    Ok(())
}

/// Cheap preflight applicability check. The generated candidate is still
/// fully constructed and kernel-validated by the execution/search path; this
/// scan avoids cloning a potentially large kernel for candidates that cannot
/// change any outcome.
fn perturbation_is_applicable(
    source: &DecisionModelWorld,
    perturbation: &ModelWorldPerturbation,
) -> bool {
    let kernel = &source.kernel;
    match perturbation {
        ModelWorldPerturbation::SwapMeasurementLabels {
            command,
            sensor_state,
            left_measurement,
            right_measurement,
        } => kernel.rows.iter().any(|row| {
            row.command == *command
                && row.state.sensor_state == *sensor_state
                && row.outcomes.iter().any(|outcome| {
                    outcome.measurement == Some(*left_measurement)
                        || outcome.measurement == Some(*right_measurement)
                })
        }),
        ModelWorldPerturbation::SwapSensorCalibration {
            sensor_state,
            left_measurement,
            right_measurement,
        } => kernel.rows.iter().any(|row| {
            row.state.sensor_state == *sensor_state
                && row.outcomes.iter().any(|outcome| {
                    outcome.measurement == Some(*left_measurement)
                        || outcome.measurement == Some(*right_measurement)
                })
        }),
        ModelWorldPerturbation::DropReadback {
            command,
            sensor_state,
            ..
        } => kernel.rows.iter().any(|row| {
            row_matches(row.state, row.command, *command, *sensor_state)
                && row
                    .outcomes
                    .iter()
                    .any(|outcome| outcome.readback.is_some())
        }),
        ModelWorldPerturbation::InjectHazard {
            command,
            sensor_state,
            ..
        } => kernel.rows.iter().any(|row| {
            row_matches(row.state, row.command, *command, *sensor_state)
                && row.outcomes.iter().any(|outcome| !outcome.hazard)
        }),
        ModelWorldPerturbation::ShiftOutcomeMass {
            state,
            command,
            source_outcome,
            target_outcome,
            amount,
        } => {
            if *command >= kernel.command_ids.len()
                || *source_outcome == *target_outcome
                || amount.is_zero()
                || state.hypothesis >= kernel.hypothesis_ids.len()
                || state.physical_state >= kernel.physical_state_count
                || state.sensor_state >= kernel.sensor_state_count
                || state.apparatus_state >= kernel.apparatus_state_count
            {
                return false;
            }
            let row = &kernel.rows[kernel.row_index(*state, *command)];
            if *source_outcome >= row.outcomes.len() || *target_outcome >= row.outcomes.len() {
                return false;
            }
            let source_mass = row.outcomes[*source_outcome].probability;
            let target_mass = row.outcomes[*target_outcome].probability;
            *amount < source_mass
                && target_mass
                    .checked_add(*amount)
                    .is_ok_and(Rational::is_probability)
        }
        ModelWorldPerturbation::DegradeApparatus {
            command,
            current_apparatus_state_at_least,
            degraded_next_state,
        } => kernel.rows.iter().any(|row| {
            row.command == *command
                && row.state.apparatus_state >= *current_apparatus_state_at_least
                && row
                    .outcomes
                    .iter()
                    .any(|outcome| outcome.next_state.apparatus_state != *degraded_next_state)
        }),
        ModelWorldPerturbation::RemapRealizedAction {
            command,
            sensor_state,
            from_action,
            ..
        } => kernel.rows.iter().any(|row| {
            row_matches(row.state, row.command, *command, *sensor_state)
                && row
                    .outcomes
                    .iter()
                    .any(|outcome| outcome.realized_action == *from_action)
        }),
    }
}

/// Search a fixed policy against a finite, deterministic perturbation family.
///
/// A policy that has no branch for a newly reachable public observation is
/// reported as invalidated. Valid counterexamples are kept with their exact
/// perturbation parameters and candidate kernel so a caller can explicitly
/// admit that immutable world into a subsequent planning run. The function
/// never silently modifies the input family or policy.
#[allow(clippy::too_many_arguments)]
pub fn search_policy_counterexamples(
    worlds: &[DecisionModelWorld],
    nominal_world_id: &str,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    policy: &DecisionPlan,
    horizon: usize,
    budgets: DecisionBudgets,
    specs: &[PerturbationSpec],
) -> Result<CounterexampleSearchReport, String> {
    budgets.validate()?;
    let baseline = replay_shared_policy_across_worlds(
        worlds,
        nominal_world_id,
        table,
        protocol,
        command_costs,
        hazard_loss,
        policy,
        horizon,
    )?;
    let baseline_worst_case_total_loss = baseline.worst_case_total_loss()?;
    let mut report = CounterexampleSearchReport {
        baseline_worlds: worlds.to_vec(),
        baseline_world_ids: baseline
            .per_world
            .iter()
            .map(|world| world.world_id.clone())
            .collect(),
        baseline_worst_case_total_loss,
        baseline_worst_case_terminal_loss: baseline
            .per_world
            .iter()
            .map(|world| world.expected_terminal_loss)
            .max()
            .unwrap_or_else(Rational::zero),
        nominal_world_id: nominal_world_id.to_string(),
        loss_table: table.clone(),
        protocol,
        command_costs: command_costs.to_vec(),
        hazard_loss,
        policy: policy.clone(),
        horizon,
        budgets,
        attempted: 0,
        witnesses: Vec::new(),
        non_violating_candidates: Vec::new(),
        rejected: Vec::new(),
    };

    let mut used_candidate_ids: BTreeSet<String> =
        worlds.iter().map(|world| world.id.clone()).collect();
    for spec in specs {
        report.attempted = report
            .attempted
            .checked_add(1)
            .ok_or_else(|| "counterexample attempt counter overflow".to_string())?;
        if spec.candidate_world_id.trim().is_empty()
            || !used_candidate_ids.insert(spec.candidate_world_id.clone())
        {
            report.rejected.push(RejectedPerturbation {
                spec: spec.clone(),
                reason: "candidate world ID is empty or already present in the family".into(),
            });
            continue;
        }
        let Some(source) = worlds.iter().find(|world| world.id == spec.source_world_id) else {
            report.rejected.push(RejectedPerturbation {
                spec: spec.clone(),
                reason: "source world is absent from the supplied model family".into(),
            });
            continue;
        };
        let candidate_world = match apply_model_world_perturbation(
            source,
            &spec.candidate_world_id,
            &spec.perturbation,
        ) {
            Ok(candidate) => candidate,
            Err(reason) => {
                report.rejected.push(RejectedPerturbation {
                    spec: spec.clone(),
                    reason,
                });
                continue;
            }
        };

        let mut expanded_worlds = worlds.to_vec();
        expanded_worlds.push(candidate_world.clone());
        let outcome = match replay_shared_policy_across_worlds(
            &expanded_worlds,
            nominal_world_id,
            table,
            protocol,
            command_costs,
            hazard_loss,
            policy,
            horizon,
        ) {
            Err(reason) if is_policy_invalidated_error(&reason) => {
                CounterexampleOutcome::PolicyInvalidated { reason }
            }
            Err(reason) => {
                report.rejected.push(RejectedPerturbation {
                    spec: spec.clone(),
                    reason: format!("candidate replay was inconclusive: {reason}"),
                });
                continue;
            }
            Ok(replay) => {
                let Some(metrics) = replay
                    .per_world
                    .iter()
                    .find(|metrics| metrics.world_id == candidate_world.id)
                    .cloned()
                else {
                    report.rejected.push(RejectedPerturbation {
                        spec: spec.clone(),
                        reason: "replay receipt omitted the admitted candidate world".into(),
                    });
                    continue;
                };

                if metrics.hazard_probability > budgets.max_hazard_probability {
                    CounterexampleOutcome::HazardBudgetViolated {
                        metrics,
                        allowed_hazard_probability: budgets.max_hazard_probability,
                    }
                } else if metrics.abstention_probability > budgets.max_abstention_probability {
                    CounterexampleOutcome::AbstentionBudgetViolated {
                        metrics,
                        allowed_abstention_probability: budgets.max_abstention_probability,
                    }
                } else if metrics.expected_terminal_loss > report.baseline_worst_case_terminal_loss
                {
                    CounterexampleOutcome::DecisionLossAmplified {
                        metrics,
                        baseline_worst_case_terminal_loss: report.baseline_worst_case_terminal_loss,
                    }
                } else {
                    CounterexampleOutcome::NoViolation { metrics }
                }
            }
        };

        let witness = CounterexampleWitness {
            spec: spec.clone(),
            candidate_world,
            outcome,
        };
        if witness.outcome.class() == CounterexampleClass::NoViolation {
            report.non_violating_candidates.push(witness);
        } else {
            report.witnesses.push(witness);
        }
    }

    report.witnesses.sort_by(compare_witness_severity);
    report.non_violating_candidates.sort_by(|left, right| {
        left.spec
            .candidate_world_id
            .cmp(&right.spec.candidate_world_id)
    });
    report.rejected.sort_by(|left, right| {
        left.spec
            .candidate_world_id
            .cmp(&right.spec.candidate_world_id)
    });
    Ok(report)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundedGridCounterexampleReport {
    pub grid: BoundedPerturbationGridReport,
    pub search: CounterexampleSearchReport,
}

/// Generate a bounded deterministic perturbation grid and attack one frozen
/// policy with every valid candidate. Grid generation and replay receipts are
/// returned together so the caller can inspect skipped combinations and
/// distinguish search-domain exhaustion from policy robustness.
#[allow(clippy::too_many_arguments)]
pub fn search_policy_against_bounded_grid(
    worlds: &[DecisionModelWorld],
    nominal_world_id: &str,
    table: &DecisionLossTable,
    protocol: ObservationProtocol,
    command_costs: &[Rational],
    hazard_loss: Rational,
    policy: &DecisionPlan,
    horizon: usize,
    budgets: DecisionBudgets,
    grid: &BoundedPerturbationGrid,
    max_attempts: usize,
) -> Result<BoundedGridCounterexampleReport, String> {
    let grid_report = generate_bounded_perturbation_grid(worlds, grid, max_attempts)?;
    let search_report = search_policy_counterexamples(
        worlds,
        nominal_world_id,
        table,
        protocol,
        command_costs,
        hazard_loss,
        policy,
        horizon,
        budgets,
        &grid_report.specs,
    )?;
    Ok(BoundedGridCounterexampleReport {
        grid: grid_report,
        search: search_report,
    })
}

/// Distinguish a policy that cannot represent newly reachable behavior
/// from a replay failure caused by malformed inputs or arithmetic limits.
fn is_policy_invalidated_error(reason: &str) -> bool {
    [
        "shared policy must contain exactly one branch per reachable public observation",
        "observed hazard branch must terminate immediately",
        "non-hazard observation cannot use a hazard terminal",
        "shared policy exceeds its finite horizon",
        "shared policy references an invalid command",
        "terminal policy references an invalid decision",
    ]
    .iter()
    .any(|prefix| reason.starts_with(prefix))
}

#[cfg(test)]
mod error_classification_tests {
    use super::*;

    #[test]
    fn only_policy_contract_failures_are_reported_as_policy_invalidation() {
        assert!(is_policy_invalidated_error(
            "shared policy must contain exactly one branch per reachable public observation"
        ));
        assert!(is_policy_invalidated_error(
            "observed hazard branch must terminate immediately"
        ));
        assert!(!is_policy_invalidated_error(
            "rational multiplication numerator overflow"
        ));
        assert!(!is_policy_invalidated_error(
            "model worlds must share dimensions"
        ));
    }
}

fn compare_witness_severity(
    left: &CounterexampleWitness,
    right: &CounterexampleWitness,
) -> Ordering {
    right
        .outcome
        .class()
        .cmp(&left.outcome.class())
        .then_with(|| {
            let (Some(a), Some(b)) = (left.outcome.metrics(), right.outcome.metrics()) else {
                return Ordering::Equal;
            };
            match left.outcome.class() {
                CounterexampleClass::HazardBudgetViolated => b
                    .hazard_probability
                    .cmp(&a.hazard_probability)
                    .then_with(|| b.expected_terminal_loss.cmp(&a.expected_terminal_loss)),
                CounterexampleClass::AbstentionBudgetViolated => b
                    .abstention_probability
                    .cmp(&a.abstention_probability)
                    .then_with(|| b.expected_terminal_loss.cmp(&a.expected_terminal_loss)),
                CounterexampleClass::DecisionLossAmplified => {
                    b.expected_terminal_loss.cmp(&a.expected_terminal_loss)
                }
                CounterexampleClass::PolicyInvalidated | CounterexampleClass::NoViolation => {
                    Ordering::Equal
                }
            }
        })
        .then_with(|| {
            left.spec
                .candidate_world_id
                .cmp(&right.spec.candidate_world_id)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BeliefMass, KernelRow, LatentState, Rational, TransitionOutcome};

    fn half() -> Rational {
        Rational::new(1, 2).unwrap()
    }

    fn world() -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(hypothesis),
                    hazard: false,
                }],
            });
            rows.push(KernelRow {
                state,
                command: 1,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(hypothesis),
                    hazard: false,
                }],
            });
        }
        DecisionModelWorld {
            id: "nominal".into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["read-0".into(), "read-1".into()],
                realized_action_ids: vec!["executed".into(), "dropped".into()],
                readback_ids: vec!["ok".into()],
                measurement_ids: vec!["zero".into(), "one".into()],
                initial_belief: vec![
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 0,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 1,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn world_with_two_apparatus_states() -> DecisionModelWorld {
        let mut source = world();
        source.kernel.apparatus_state_count = 2;
        source.kernel.rows.clear();
        for hypothesis in 0..2 {
            for apparatus_state in 0..2 {
                let state = LatentState {
                    hypothesis,
                    physical_state: 0,
                    sensor_state: 0,
                    apparatus_state,
                };
                for command in 0..2 {
                    source.kernel.rows.push(KernelRow {
                        state,
                        command,
                        outcomes: vec![TransitionOutcome {
                            probability: Rational::one(),
                            next_state: state,
                            realized_action: 0,
                            readback: Some(0),
                            measurement: Some(hypothesis),
                            hazard: false,
                        }],
                    });
                }
            }
        }
        source.kernel.validate().unwrap();
        source
    }

    fn instrument_world(id: &str) -> DecisionModelWorld {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            let state = LatentState {
                hypothesis,
                physical_state: 0,
                sensor_state: 0,
                apparatus_state: 0,
            };
            rows.push(KernelRow {
                state,
                command: 0,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(0),
                    hazard: false,
                }],
            });
            rows.push(KernelRow {
                state,
                command: 1,
                outcomes: vec![TransitionOutcome {
                    probability: Rational::one(),
                    next_state: state,
                    realized_action: 0,
                    readback: Some(0),
                    measurement: Some(hypothesis),
                    hazard: false,
                }],
            });
        }
        DecisionModelWorld {
            id: id.into(),
            kernel: ActionRealizationKernel {
                hypothesis_ids: vec!["h0".into(), "h1".into()],
                physical_state_count: 1,
                sensor_state_count: 1,
                apparatus_state_count: 1,
                command_ids: vec!["calibration-probe".into(), "science-read".into()],
                realized_action_ids: vec!["executed".into()],
                readback_ids: vec!["ok".into()],
                measurement_ids: vec!["zero".into(), "one".into()],
                initial_belief: vec![
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 0,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                    BeliefMass {
                        state: LatentState {
                            hypothesis: 1,
                            physical_state: 0,
                            sensor_state: 0,
                            apparatus_state: 0,
                        },
                        mass: half(),
                    },
                ],
                rows,
            },
        }
    }

    fn table() -> DecisionLossTable {
        DecisionLossTable {
            decision_ids: vec!["claim-h0".into(), "claim-h1".into()],
            losses: vec![
                vec![
                    vec![vec![vec![Rational::zero()]]],
                    vec![vec![vec![Rational::one()]]],
                ],
                vec![
                    vec![vec![vec![Rational::one()]]],
                    vec![vec![vec![Rational::zero()]]],
                ],
            ],
            abstention_loss: Rational::one(),
        }
    }

    fn policy() -> DecisionPlan {
        let first = crate::PublicObservation {
            hazard: false,
            readback: crate::ObservationField::Value(0),
            measurement: crate::ObservationField::Value(0),
        };
        let second = crate::PublicObservation {
            measurement: crate::ObservationField::Value(1),
            ..first
        };
        DecisionPlan::Experiment {
            command: 0,
            branches: vec![
                super::super::finite_horizon::DecisionBranch {
                    observation: first,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(0) }),
                },
                super::super::finite_horizon::DecisionBranch {
                    observation: second,
                    plan: Box::new(DecisionPlan::TerminalDecision { decision: Some(1) }),
                },
            ],
        }
    }

    fn spec(id: &str, perturbation: ModelWorldPerturbation) -> PerturbationSpec {
        PerturbationSpec {
            source_world_id: "nominal".into(),
            candidate_world_id: id.into(),
            perturbation,
        }
    }

    #[test]
    fn bounded_grid_generation_is_deterministic_and_emits_only_valid_kernels() {
        let mut first = world();
        first.id = "world-a".into();
        let mut second = world();
        second.id = "world-b".into();
        second.kernel.measurement_ids = vec!["m0".into(), "m1".into()];
        let grid = BoundedPerturbationGrid {
            split_probabilities: vec![Rational::new(1, 10).unwrap()],
            outcome_mass_shifts: vec![],
            include_measurement_label_swaps: true,
            include_readback_dropout: true,
            include_hazard_injection: true,
            include_outcome_mass_shifts: false,
            include_apparatus_degradation: false,
            include_realized_action_remapping: true,
            include_sensor_scoped_variants: true,
        };
        let forward =
            generate_bounded_perturbation_grid(&[first.clone(), second.clone()], &grid, 100)
                .unwrap();
        let reversed =
            generate_bounded_perturbation_grid(&[second.clone(), first.clone()], &grid, 100)
                .unwrap();
        assert_eq!(forward, reversed);
        assert!(!forward.specs.is_empty());

        let mut candidate_ids = BTreeSet::new();
        for spec in &forward.specs {
            assert!(candidate_ids.insert(spec.candidate_world_id.as_str()));
            let source = [&first, &second]
                .into_iter()
                .find(|world| world.id == spec.source_world_id)
                .unwrap();
            let candidate = apply_model_world_perturbation(
                source,
                &spec.candidate_world_id,
                &spec.perturbation,
            )
            .unwrap();
            candidate.kernel.validate().unwrap();
        }
        assert_eq!(
            forward.attempted,
            forward.specs.len() + forward.skipped_inapplicable
        );
    }

    #[test]
    fn bounded_grid_fails_closed_on_attempt_caps_and_invalid_probabilities() {
        let source = world();
        let grid = BoundedPerturbationGrid {
            split_probabilities: vec![Rational::new(1, 10).unwrap()],
            outcome_mass_shifts: vec![],
            include_measurement_label_swaps: true,
            include_readback_dropout: false,
            include_hazard_injection: false,
            include_outcome_mass_shifts: false,
            include_apparatus_degradation: false,
            include_realized_action_remapping: false,
            include_sensor_scoped_variants: false,
        };
        assert!(generate_bounded_perturbation_grid(&[source.clone()], &grid, 1).is_err());

        let invalid = BoundedPerturbationGrid {
            split_probabilities: vec![Rational::one()],
            ..grid
        };
        assert!(generate_bounded_perturbation_grid(&[source], &invalid, 100).is_err());
    }

    fn base_inputs() -> (Vec<DecisionModelWorld>, DecisionLossTable, Vec<Rational>) {
        (
            vec![world()],
            table(),
            vec![Rational::new(1, 10).unwrap(), Rational::new(1, 10).unwrap()],
        )
    }

    #[test]
    fn measurement_inversion_is_a_decision_loss_counterexample() {
        let (worlds, table, costs) = base_inputs();
        let perturbation = spec(
            "sensor-inverted",
            ModelWorldPerturbation::SwapMeasurementLabels {
                command: 0,
                sensor_state: 0,
                left_measurement: 0,
                right_measurement: 1,
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[perturbation],
        )
        .unwrap();
        let counterexample = report.strongest_counterexample().unwrap();
        assert_eq!(
            counterexample.outcome.class(),
            CounterexampleClass::DecisionLossAmplified
        );
        match &counterexample.outcome {
            CounterexampleOutcome::DecisionLossAmplified { metrics, .. } => {
                assert_eq!(metrics.expected_terminal_loss, Rational::one());
            }
            _ => panic!("measurement inversion should amplify terminal decision loss"),
        }
        assert_eq!(
            report.baseline_worst_case_total_loss,
            Rational::new(1, 10).unwrap()
        );
        assert_eq!(report.attempted, 1);
        assert_eq!(report.witnesses.len(), 1);
    }

    #[test]
    fn plan_attack_expand_replan_recovers_from_sensor_calibration_inversion() {
        let nominal = instrument_world("nominal");
        let table = table();
        let protocol = ObservationProtocol::fully_observed();
        let costs = vec![Rational::new(1, 10).unwrap(), Rational::new(1, 20).unwrap()];
        let budgets = DecisionBudgets {
            max_hazard_probability: Rational::zero(),
            max_abstention_probability: Rational::one(),
        };
        let nominal_result = super::super::finite_horizon::optimize_finite_horizon_decisions(
            &nominal.kernel,
            &table,
            protocol,
            &costs,
            Rational::one(),
            &super::super::finite_horizon::DecisionSolverConfig {
                horizon: 1,
                max_memo_states: 20_000,
                max_frontier_points: 20_000,
                max_generated_combinations: 200_000,
            },
        )
        .unwrap();
        let nominal_policy = nominal_result
            .minimum_loss_under(budgets)
            .unwrap()
            .unwrap()
            .plan
            .clone();
        assert!(matches!(
            &nominal_policy,
            DecisionPlan::Experiment { command: 1, .. }
        ));

        let attack_grid = BoundedPerturbationGrid {
            split_probabilities: Vec::new(),
            outcome_mass_shifts: Vec::new(),
            include_measurement_label_swaps: true,
            include_readback_dropout: false,
            include_hazard_injection: false,
            include_outcome_mass_shifts: false,
            include_apparatus_degradation: false,
            include_realized_action_remapping: false,
            include_sensor_scoped_variants: false,
        };
        let attack = search_policy_against_bounded_grid(
            &[nominal.clone()],
            "nominal",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &nominal_policy,
            1,
            budgets,
            &attack_grid,
            16,
        )
        .unwrap();
        assert_eq!(attack.grid.specs.len(), 1);
        assert_eq!(attack.grid.attempted, 1);
        let witness = attack.search.strongest_counterexample().unwrap();
        assert_eq!(
            witness.outcome.class(),
            CounterexampleClass::DecisionLossAmplified
        );
        match &witness.outcome {
            CounterexampleOutcome::DecisionLossAmplified { metrics, .. } => {
                assert_eq!(metrics.expected_terminal_loss, Rational::one());
            }
            _ => panic!("sensor inversion should invalidate nominal decision quality"),
        }

        // Admit the immutable, validated counterexample explicitly and re-plan.
        let expanded_family = vec![nominal, witness.candidate_world.clone()];
        let robust = super::super::robust_finite_horizon::optimize_shared_policy_decisions(
            &expanded_family,
            "nominal",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &super::super::finite_horizon::DecisionSolverConfig {
                horizon: 2,
                max_memo_states: 20_000,
                max_frontier_points: 20_000,
                max_generated_combinations: 200_000,
            },
        )
        .unwrap();
        let replanned = robust
            .minimum_worst_case_total_loss_under(budgets)
            .unwrap()
            .unwrap();
        assert_eq!(replanned.worst_case_terminal_loss(), Rational::zero());
        assert_eq!(
            replanned.worst_case_total_loss().unwrap(),
            Rational::new(3, 20).unwrap()
        );

        let replay = replay_shared_policy_across_worlds(
            &expanded_family,
            "nominal",
            &table,
            protocol,
            &costs,
            Rational::one(),
            &replanned.plan,
            2,
        )
        .unwrap();
        assert_eq!(replay.per_world, replanned.per_world);
    }

    #[test]
    fn newly_reachable_missing_readback_invalidates_the_fixed_policy() {
        let (worlds, table, costs) = base_inputs();
        let perturbation = spec(
            "readback-dropped",
            ModelWorldPerturbation::DropReadback {
                command: 0,
                sensor_state: None,
                dropout_probability: Rational::new(1, 10).unwrap(),
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[perturbation],
        )
        .unwrap();
        assert_eq!(
            report.strongest_counterexample().unwrap().outcome.class(),
            CounterexampleClass::PolicyInvalidated
        );
    }

    #[test]
    fn hazard_injection_invalidates_policy_when_it_lacks_hazard_fallback() {
        let (worlds, table, costs) = base_inputs();
        let perturbation = spec(
            "hazard-shifted",
            ModelWorldPerturbation::InjectHazard {
                command: 0,
                sensor_state: None,
                hazard_probability: Rational::new(1, 20).unwrap(),
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::one(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[perturbation],
        )
        .unwrap();
        assert_eq!(
            report.strongest_counterexample().unwrap().outcome.class(),
            CounterexampleClass::PolicyInvalidated
        );
    }

    #[test]
    fn outcome_mass_shift_preserves_exact_row_normalization() {
        let source = world();
        let candidate = apply_model_world_perturbation(
            &source,
            "shifted",
            &ModelWorldPerturbation::ShiftOutcomeMass {
                state: source.kernel.initial_belief[0].state,
                command: 0,
                source_outcome: 0,
                target_outcome: 0,
                amount: Rational::new(1, 10).unwrap(),
            },
        );
        assert!(candidate.is_err());

        let mut split = source.clone();
        let next_state = split.kernel.rows[0].outcomes[0].next_state;
        split.kernel.rows[0].outcomes.push(TransitionOutcome {
            probability: Rational::new(1, 5).unwrap(),
            next_state,
            realized_action: 0,
            readback: Some(0),
            measurement: Some(1),
            hazard: false,
        });
        split.kernel.rows[0].outcomes[0].probability = Rational::new(4, 5).unwrap();
        let candidate = apply_model_world_perturbation(
            &split,
            "shifted-two-outcome",
            &ModelWorldPerturbation::ShiftOutcomeMass {
                state: split.kernel.initial_belief[0].state,
                command: 0,
                source_outcome: 0,
                target_outcome: 1,
                amount: Rational::new(1, 10).unwrap(),
            },
        )
        .unwrap();
        candidate.kernel.validate().unwrap();
        assert_eq!(
            candidate.kernel.rows[0].outcomes[0].probability,
            Rational::new(7, 10).unwrap()
        );
        assert_eq!(
            candidate.kernel.rows[0].outcomes[1].probability,
            Rational::new(3, 10).unwrap()
        );
    }

    #[test]
    fn readback_dropout_and_hazard_mutations_preserve_kernel_mass() {
        let source = world();
        let dropout = apply_model_world_perturbation(
            &source,
            "dropout",
            &ModelWorldPerturbation::DropReadback {
                command: 0,
                sensor_state: None,
                dropout_probability: Rational::new(1, 10).unwrap(),
            },
        )
        .unwrap();
        dropout.kernel.validate().unwrap();
        assert_eq!(dropout.kernel.rows[0].outcomes.len(), 2);
        assert_eq!(
            dropout.kernel.rows[0].outcomes[0]
                .probability
                .checked_add(dropout.kernel.rows[0].outcomes[1].probability)
                .unwrap(),
            Rational::one()
        );

        let hazard = apply_model_world_perturbation(
            &source,
            "hazard",
            &ModelWorldPerturbation::InjectHazard {
                command: 0,
                sensor_state: None,
                hazard_probability: Rational::new(1, 20).unwrap(),
            },
        )
        .unwrap();
        hazard.kernel.validate().unwrap();
        assert_eq!(hazard.kernel.rows[0].outcomes.len(), 2);
        assert_eq!(
            hazard.kernel.rows[0]
                .outcomes
                .iter()
                .filter(|outcome| outcome.hazard)
                .count(),
            1
        );
    }

    #[test]
    fn apparatus_degradation_and_hidden_action_remap_are_exact() {
        let source = world_with_two_apparatus_states();
        let degraded = apply_model_world_perturbation(
            &source,
            "degraded",
            &ModelWorldPerturbation::DegradeApparatus {
                command: 0,
                current_apparatus_state_at_least: 0,
                degraded_next_state: 1,
            },
        )
        .unwrap();
        degraded.kernel.validate().unwrap();
        assert!(
            degraded
                .kernel
                .rows
                .iter()
                .filter(|row| row.command == 0 && row.state.apparatus_state == 0)
                .all(|row| row
                    .outcomes
                    .iter()
                    .all(|outcome| outcome.next_state.apparatus_state == 1))
        );

        let remapped = apply_model_world_perturbation(
            &source,
            "action-remapped",
            &ModelWorldPerturbation::RemapRealizedAction {
                command: 0,
                sensor_state: None,
                from_action: 0,
                to_action: 1,
            },
        )
        .unwrap();
        remapped.kernel.validate().unwrap();
        assert!(
            remapped
                .kernel
                .rows
                .iter()
                .filter(|row| row.command == 0)
                .all(|row| row
                    .outcomes
                    .iter()
                    .all(|outcome| outcome.realized_action == 1))
        );
    }

    #[test]
    fn non_violating_hidden_action_change_is_retained_as_a_control() {
        let (worlds, table, costs) = base_inputs();
        let action_remap = spec(
            "hidden-action-remap",
            ModelWorldPerturbation::RemapRealizedAction {
                command: 0,
                sensor_state: None,
                from_action: 0,
                to_action: 1,
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[action_remap],
        )
        .unwrap();
        assert_eq!(report.witnesses.len(), 0);
        assert_eq!(report.non_violating_candidates.len(), 1);
        assert_eq!(
            report.non_violating_candidates[0].outcome.class(),
            CounterexampleClass::NoViolation
        );
        assert_eq!(report.baseline_worlds, worlds);
        assert_eq!(report.loss_table, table);
        assert_eq!(report.policy, policy());
        assert_eq!(report.command_costs, costs);
    }

    #[test]
    fn repeated_candidate_id_is_rejected_within_one_search_batch() {
        let (worlds, table, costs) = base_inputs();
        let specs = [
            spec(
                "same-candidate",
                ModelWorldPerturbation::SwapMeasurementLabels {
                    command: 0,
                    sensor_state: 0,
                    left_measurement: 0,
                    right_measurement: 1,
                },
            ),
            spec(
                "same-candidate",
                ModelWorldPerturbation::InjectHazard {
                    command: 0,
                    sensor_state: None,
                    hazard_probability: Rational::new(1, 10).unwrap(),
                },
            ),
        ];
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::one(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &specs,
        )
        .unwrap();
        assert_eq!(report.attempted, 2);
        assert_eq!(report.witnesses.len(), 1);
        assert_eq!(report.rejected.len(), 1);
    }

    #[test]
    fn generator_rejects_invalid_probability_and_missing_source() {
        let source = world();
        assert!(
            apply_model_world_perturbation(
                &source,
                "bad-probability",
                &ModelWorldPerturbation::InjectHazard {
                    command: 0,
                    sensor_state: None,
                    hazard_probability: Rational::one(),
                },
            )
            .is_err()
        );
        assert!(
            apply_model_world_perturbation(
                &source,
                "bad-measurement",
                &ModelWorldPerturbation::SwapMeasurementLabels {
                    command: 0,
                    sensor_state: 0,
                    left_measurement: 0,
                    right_measurement: 9,
                },
            )
            .is_err()
        );
    }

    #[test]
    fn search_rejects_duplicate_candidate_ids_and_records_rejections() {
        let (worlds, table, costs) = base_inputs();
        let duplicate = spec(
            "nominal",
            ModelWorldPerturbation::SwapMeasurementLabels {
                command: 0,
                sensor_state: 0,
                left_measurement: 0,
                right_measurement: 1,
            },
        );
        let report = search_policy_counterexamples(
            &worlds,
            "nominal",
            &table,
            ObservationProtocol::fully_observed(),
            &costs,
            Rational::zero(),
            &policy(),
            1,
            DecisionBudgets {
                max_hazard_probability: Rational::zero(),
                max_abstention_probability: Rational::one(),
            },
            &[duplicate],
        )
        .unwrap();
        assert_eq!(report.attempted, 1);
        assert_eq!(report.rejected.len(), 1);
        assert!(report.witnesses.is_empty());
    }
}
