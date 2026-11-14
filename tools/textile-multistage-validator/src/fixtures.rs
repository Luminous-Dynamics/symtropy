use std::collections::BTreeSet;

use crate::{
    Assignment, Interval, Machine, Operation, Operator, Precedence, Problem, Schedule, SetupRule,
};

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn machine(id: &str, capability: &str) -> Machine {
    Machine {
        id: id.to_owned(),
        capabilities: set(&[capability]),
        availability: vec![Interval {
            start_tick: 0,
            end_tick: 30,
        }],
        initial_family: "woven".to_owned(),
        setup_rules: vec![SetupRule {
            from_family: "woven".to_owned(),
            to_family: "woven".to_owned(),
            duration_ticks: 0,
            required_operator_skill: None,
            required_operator_certification: None,
        }],
    }
}

fn operator(id: &str, skill: &str, certification: &str) -> Operator {
    Operator {
        id: id.to_owned(),
        skills: set(&[skill]),
        certifications: set(&[certification]),
        availability: vec![Interval {
            start_tick: 0,
            end_tick: 30,
        }],
    }
}

fn operation(
    id: &str,
    job_id: &str,
    stage_index: u32,
    duration_ticks: u64,
    capability: &str,
    skill: &str,
    certification: &str,
) -> Operation {
    Operation {
        id: id.to_owned(),
        job_id: job_id.to_owned(),
        stage_index,
        duration_ticks,
        release_tick: 0,
        due_tick: Some(25),
        required_capability: capability.to_owned(),
        required_skill: skill.to_owned(),
        required_certification: Some(certification.to_owned()),
        product_family: "woven".to_owned(),
    }
}

/// Two jobs routed through cutting, sewing, finishing and inspection.
pub fn two_job_four_stage_case() -> (Problem, Schedule) {
    let problem = Problem {
        name: "two-job-four-stage-woven-garment".to_owned(),
        machines: vec![
            machine("cut-line", "cutting"),
            machine("sew-line", "sewing"),
            machine("finish-line", "finishing"),
            machine("qa-bench", "inspection"),
        ],
        operators: vec![
            operator("cutter", "cutting", "cut-qualified"),
            operator("sewer", "sewing", "sew-qualified"),
            operator("finisher", "finishing", "finish-qualified"),
            operator("inspector", "inspection", "inspect-qualified"),
        ],
        operations: vec![
            operation(
                "cut-a",
                "job-a",
                0,
                3,
                "cutting",
                "cutting",
                "cut-qualified",
            ),
            operation("sew-a", "job-a", 1, 5, "sewing", "sewing", "sew-qualified"),
            operation(
                "finish-a",
                "job-a",
                2,
                2,
                "finishing",
                "finishing",
                "finish-qualified",
            ),
            operation(
                "inspect-a",
                "job-a",
                3,
                1,
                "inspection",
                "inspection",
                "inspect-qualified",
            ),
            operation(
                "cut-b",
                "job-b",
                0,
                2,
                "cutting",
                "cutting",
                "cut-qualified",
            ),
            operation("sew-b", "job-b", 1, 4, "sewing", "sewing", "sew-qualified"),
            operation(
                "finish-b",
                "job-b",
                2,
                2,
                "finishing",
                "finishing",
                "finish-qualified",
            ),
            operation(
                "inspect-b",
                "job-b",
                3,
                1,
                "inspection",
                "inspection",
                "inspect-qualified",
            ),
        ],
        precedences: vec![
            Precedence {
                before_operation_id: "cut-a".to_owned(),
                after_operation_id: "sew-a".to_owned(),
                minimum_lag_ticks: 0,
            },
            Precedence {
                before_operation_id: "sew-a".to_owned(),
                after_operation_id: "finish-a".to_owned(),
                minimum_lag_ticks: 0,
            },
            Precedence {
                before_operation_id: "finish-a".to_owned(),
                after_operation_id: "inspect-a".to_owned(),
                minimum_lag_ticks: 0,
            },
            Precedence {
                before_operation_id: "cut-b".to_owned(),
                after_operation_id: "sew-b".to_owned(),
                minimum_lag_ticks: 0,
            },
            Precedence {
                before_operation_id: "sew-b".to_owned(),
                after_operation_id: "finish-b".to_owned(),
                minimum_lag_ticks: 0,
            },
            Precedence {
                before_operation_id: "finish-b".to_owned(),
                after_operation_id: "inspect-b".to_owned(),
                minimum_lag_ticks: 0,
            },
        ],
    };

    let schedule = Schedule {
        assignments: vec![
            Assignment {
                operation_id: "cut-a".to_owned(),
                machine_id: "cut-line".to_owned(),
                operator_id: "cutter".to_owned(),
                start_tick: 0,
                finish_tick: 3,
            },
            Assignment {
                operation_id: "sew-a".to_owned(),
                machine_id: "sew-line".to_owned(),
                operator_id: "sewer".to_owned(),
                start_tick: 3,
                finish_tick: 8,
            },
            Assignment {
                operation_id: "finish-a".to_owned(),
                machine_id: "finish-line".to_owned(),
                operator_id: "finisher".to_owned(),
                start_tick: 8,
                finish_tick: 10,
            },
            Assignment {
                operation_id: "inspect-a".to_owned(),
                machine_id: "qa-bench".to_owned(),
                operator_id: "inspector".to_owned(),
                start_tick: 10,
                finish_tick: 11,
            },
            Assignment {
                operation_id: "cut-b".to_owned(),
                machine_id: "cut-line".to_owned(),
                operator_id: "cutter".to_owned(),
                start_tick: 4,
                finish_tick: 6,
            },
            Assignment {
                operation_id: "sew-b".to_owned(),
                machine_id: "sew-line".to_owned(),
                operator_id: "sewer".to_owned(),
                start_tick: 8,
                finish_tick: 12,
            },
            Assignment {
                operation_id: "finish-b".to_owned(),
                machine_id: "finish-line".to_owned(),
                operator_id: "finisher".to_owned(),
                start_tick: 12,
                finish_tick: 14,
            },
            Assignment {
                operation_id: "inspect-b".to_owned(),
                machine_id: "qa-bench".to_owned(),
                operator_id: "inspector".to_owned(),
                start_tick: 14,
                finish_tick: 15,
            },
        ],
        setup_reservations: Vec::new(),
    };

    (problem, schedule)
}
