//! Clean-room pilot equations for STEP7 (C2-DTLZ2, M=2, D=5)
//! and STEP24 (C1-DTLZ1, M=3, D=5).
//!
//! This is a formula/constraint-sign pilot, not a STEP optimizer or a full suite adapter.
//! Constraint margins are positive when feasible; archive admission still requires a
//! separate evidence gate and independent evaluation.

use std::f64::consts::PI;

const BOUND_TOLERANCE: f64 = 1.0e-12;
const FEASIBILITY_TOLERANCE: f64 = 1.0e-12;
const C2_RADIUS: f64 = 0.1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPilotTask {
    Step7C2Dtlz2M2D5,
    Step24C1Dtlz1M3D5,
}

impl StepPilotTask {
    pub fn task_id(self) -> &'static str {
        match self {
            Self::Step7C2Dtlz2M2D5 => "STEP7",
            Self::Step24C1Dtlz1M3D5 => "STEP24",
        }
    }

    pub fn problem_name(self) -> &'static str {
        match self {
            Self::Step7C2Dtlz2M2D5 => "C2-DTLZ2",
            Self::Step24C1Dtlz1M3D5 => "C1-DTLZ1",
        }
    }

    pub fn objective_count(self) -> usize {
        match self {
            Self::Step7C2Dtlz2M2D5 => 2,
            Self::Step24C1Dtlz1M3D5 => 3,
        }
    }

    pub fn decision_dimension(self) -> usize {
        5
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StepPilotEvaluation {
    pub task_id: &'static str,
    pub problem_name: &'static str,
    pub decision_vector: Vec<f64>,
    pub objectives: Vec<f64>,
    /// Each margin uses the explicit convention margin >= 0 means feasible.
    pub constraint_margins: Vec<f64>,
    pub aggregate_violation: f64,
    pub feasible: bool,
}

pub fn evaluate_step_pilot(
    task: StepPilotTask,
    decision_vector: &[f64],
) -> Result<StepPilotEvaluation, String> {
    validate_decision_vector(task, decision_vector)?;
    let (objectives, constraint_margins) = match task {
        StepPilotTask::Step7C2Dtlz2M2D5 => evaluate_c2_dtlz2(decision_vector),
        StepPilotTask::Step24C1Dtlz1M3D5 => evaluate_c1_dtlz1(decision_vector),
    };
    if objectives
        .iter()
        .chain(&constraint_margins)
        .any(|value| !value.is_finite())
    {
        return Err(format!(
            "{}: formula evaluation produced a non-finite value",
            task.task_id()
        ));
    }
    if constraint_margins.is_empty() {
        return Err(format!(
            "{}: at least one constraint is required",
            task.task_id()
        ));
    }

    let aggregate_violation = constraint_margins
        .iter()
        .map(|margin| (-margin).max(0.0))
        .sum::<f64>();
    if !aggregate_violation.is_finite() {
        return Err(format!(
            "{}: aggregate constraint violation overflowed",
            task.task_id()
        ));
    }
    let feasible = constraint_margins
        .iter()
        .all(|margin| *margin >= -FEASIBILITY_TOLERANCE);

    Ok(StepPilotEvaluation {
        task_id: task.task_id(),
        problem_name: task.problem_name(),
        decision_vector: decision_vector.to_vec(),
        objectives,
        constraint_margins,
        aggregate_violation,
        feasible,
    })
}

fn validate_decision_vector(task: StepPilotTask, values: &[f64]) -> Result<(), String> {
    if values.len() != task.decision_dimension() {
        return Err(format!(
            "{}: expected {} decision variables, found {}",
            task.task_id(),
            task.decision_dimension(),
            values.len()
        ));
    }
    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(format!("{}: x{} must be finite", task.task_id(), index + 1));
        }
        if *value < -BOUND_TOLERANCE || *value > 1.0 + BOUND_TOLERANCE {
            return Err(format!(
                "{}: x{} is outside [0, 1]",
                task.task_id(),
                index + 1
            ));
        }
    }
    Ok(())
}

/// STEP7: C2-DTLZ2, M=2, D=5, using the paper's r=0.1 setting.
fn evaluate_c2_dtlz2(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let g = x[1..]
        .iter()
        .map(|value| (value - 0.5).powi(2))
        .sum::<f64>();
    let radius = 1.0 + g;
    let angle = 0.5 * PI * x[0];
    let objectives = vec![radius * angle.cos(), radius * angle.sin()];

    // Feasible set: union of radius-r balls around each unit objective axis and
    // the centre (1/sqrt(M), ..., 1/sqrt(M)). This sign matches independent
    // published formulations and the three feasible components in the source figure.
    let mut minimum_squared_distance = f64::INFINITY;
    for axis in 0..objectives.len() {
        let distance = objectives
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let centre = if index == axis { 1.0 } else { 0.0 };
                (value - centre).powi(2)
            })
            .sum::<f64>();
        minimum_squared_distance = minimum_squared_distance.min(distance);
    }
    let centre = 1.0 / (objectives.len() as f64).sqrt();
    let centre_distance = objectives
        .iter()
        .map(|value| (value - centre).powi(2))
        .sum::<f64>();
    minimum_squared_distance = minimum_squared_distance.min(centre_distance);

    let margin = C2_RADIUS.powi(2) - minimum_squared_distance;
    (objectives, vec![margin])
}

/// STEP24: C1-DTLZ1, M=3, D=5. The standard DTLZ1 g term and published
/// C1-DTLZ1 inequality are returned as a positive-feasible margin.
fn evaluate_c1_dtlz1(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let objectives_count = 3_usize;
    let k = x.len() - objectives_count + 1;
    let g = 100.0
        * (k as f64
            + x[objectives_count - 1..]
                .iter()
                .map(|value| (value - 0.5).powi(2) - (20.0 * PI * (value - 0.5)).cos())
                .sum::<f64>());
    let scale = 0.5 * (1.0 + g);
    let mut objectives = Vec::with_capacity(objectives_count);

    for objective_index in 0..objectives_count {
        let prefix_length = objectives_count - 1 - objective_index;
        let mut value = scale;
        for decision in &x[..prefix_length] {
            value *= *decision;
        }
        if objective_index > 0 {
            value *= 1.0 - x[prefix_length];
        }
        objectives.push(value);
    }

    let constraint_margin = 1.0
        - objectives[objectives_count - 1] / 0.6
        - objectives[..objectives_count - 1]
            .iter()
            .map(|value| value / 0.5)
            .sum::<f64>();
    (objectives, vec![constraint_margin])
}

pub const STEP_PILOT_CSV_HEADER: &str = "case_id,task_id,problem_name,decision_vector,objectives,constraint_margins,aggregate_violation,feasible";

pub fn evaluation_to_csv(case_id: &str, evaluation: &StepPilotEvaluation) -> String {
    let decision_vector = evaluation
        .decision_vector
        .iter()
        .map(|value| format!("{value:.12}"))
        .collect::<Vec<_>>()
        .join(";");
    let objectives = evaluation
        .objectives
        .iter()
        .map(|value| format!("{value:.12}"))
        .collect::<Vec<_>>()
        .join(";");
    let margins = evaluation
        .constraint_margins
        .iter()
        .map(|value| format!("{value:.12}"))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "{},{},{},{},{},{},{:.12},{}\n",
        csv_escape(case_id),
        evaluation.task_id,
        evaluation.problem_name,
        csv_escape(&decision_vector),
        csv_escape(&objectives),
        csv_escape(&margins),
        evaluation.aggregate_violation,
        evaluation.feasible
    )
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step7_c2_dtlz2_has_three_known_feasible_components() {
        for x1 in [0.0, 0.5, 1.0] {
            let x = [x1, 0.5, 0.5, 0.5, 0.5];
            let result = evaluate_step_pilot(StepPilotTask::Step7C2Dtlz2M2D5, &x)
                .expect("known feasible C2-DTLZ2 component");
            assert!(result.feasible);
            assert!((result.constraint_margins[0] - 0.01).abs() < 1.0e-12);
            assert_eq!(result.aggregate_violation, 0.0);
        }
    }

    #[test]
    fn step7_c2_dtlz2_rejects_the_gaps_between_feasible_components() {
        for x1 in [0.25, 0.75] {
            let x = [x1, 0.5, 0.5, 0.5, 0.5];
            let result = evaluate_step_pilot(StepPilotTask::Step7C2Dtlz2M2D5, &x)
                .expect("valid but infeasible C2-DTLZ2 point");
            assert!(!result.feasible);
            assert!(result.constraint_margins[0] < 0.0);
            assert!(result.aggregate_violation > 0.0);
        }
    }

    #[test]
    fn step24_c1_dtlz1_matches_known_objectives_and_feasible_margin() {
        let x = [0.2, 0.3, 0.5, 0.5, 0.5];
        let result = evaluate_step_pilot(StepPilotTask::Step24C1Dtlz1M3D5, &x)
            .expect("known C1-DTLZ1 point");
        assert!((result.objectives[0] - 0.03).abs() < 1.0e-12);
        assert!((result.objectives[1] - 0.07).abs() < 1.0e-12);
        assert!((result.objectives[2] - 0.4).abs() < 1.0e-12);
        assert!((result.constraint_margins[0] - (2.0 / 15.0)).abs() < 1.0e-12);
        assert!(result.feasible);
    }

    #[test]
    fn step24_c1_dtlz1_rejects_a_large_distance_function_barrier() {
        let x = [0.2, 0.3, 0.0, 0.5, 0.5];
        let result = evaluate_step_pilot(StepPilotTask::Step24C1Dtlz1M3D5, &x)
            .expect("valid but barrier-obstructed C1-DTLZ1 point");
        assert!(!result.feasible);
        assert!(result.constraint_margins[0] < 0.0);
        assert!(result.aggregate_violation > 0.0);
    }

    #[test]
    fn pilot_rejects_dimension_bounds_and_non_finite_inputs() {
        assert!(evaluate_step_pilot(StepPilotTask::Step7C2Dtlz2M2D5, &[0.0; 4]).is_err());
        assert!(
            evaluate_step_pilot(StepPilotTask::Step24C1Dtlz1M3D5, &[0.2, 0.3, 0.5, 0.5, 1.1])
                .is_err()
        );
        assert!(
            evaluate_step_pilot(
                StepPilotTask::Step7C2Dtlz2M2D5,
                &[0.2, 0.3, 0.5, 0.5, f64::NAN]
            )
            .is_err()
        );
    }
}
