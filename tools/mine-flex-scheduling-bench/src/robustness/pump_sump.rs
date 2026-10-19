use super::{is_lower_hex, sha256, Check};
use serde::Serialize;
use std::process::ExitCode;

const SUITE_ID: &str = "MINE-FLEX-PUMP-SUMP-001";
const SUITE_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
struct Bounds {
    minimum: u32,
    maximum: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PeriodInput {
    inflow_liters: Bounds,
    required_head_kpa: Bounds,
    energy_budget_wh: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Envelope {
    minimum_flow_liters_per_period: u32,
    maximum_flow_liters_per_period: u32,
    maximum_head_kpa: u32,
    operating_energy_wh_per_period: u32,
    startup_energy_wh: u32,
    startup_delay_periods: usize,
    minimum_on_periods: usize,
    minimum_off_periods: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Case {
    case_id: String,
    expected_executable: bool,
    initial_sump_liters: Bounds,
    minimum_sump_liters: u32,
    maximum_sump_liters: u32,
    commands_on: Vec<bool>,
    periods: Vec<PeriodInput>,
    envelope: Envelope,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct SumpState {
    minimum_liters: i64,
    maximum_liters: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Violation {
    code: String,
    period_index: Option<usize>,
    observed: Option<i64>,
    limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PeriodResult {
    period_index: usize,
    command_on: bool,
    operating: bool,
    inflow_liters: Bounds,
    pump_flow_liters_per_period: Bounds,
    sump_after: SumpState,
    energy_wh: u32,
    energy_budget_wh: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct CaseResult {
    case_id: String,
    expected_executable: bool,
    executable: bool,
    startup_count: usize,
    violations: Vec<Violation>,
    trajectory: Vec<PeriodResult>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Receipt {
    result: String,
    suite_id: String,
    suite_version: String,
    source_commit: String,
    generated_case_set_sha256: String,
    case_count: usize,
    period_count: usize,
    expected_controls_matched: usize,
    accepted_cases: usize,
    rejected_cases: usize,
    hard_violations_in_accepted_cases: usize,
    conservation_mismatches: usize,
    cases: Vec<CaseResult>,
    claim_ceiling: String,
}

fn violation(code: &str, period_index: Option<usize>, observed: Option<i64>, limit: Option<i64>) -> Violation {
    Violation { code: code.to_owned(), period_index, observed, limit }
}

fn validate_case_shape(case: &Case) -> Check<()> {
    if case.case_id.trim().is_empty() || case.periods.is_empty() || case.periods.len() != case.commands_on.len() {
        return Err(format!("{}: blank identity or mismatched/empty time grid", case.case_id));
    }
    if case.initial_sump_liters.minimum > case.initial_sump_liters.maximum
        || case.minimum_sump_liters > case.maximum_sump_liters
        || case.initial_sump_liters.minimum < case.minimum_sump_liters
        || case.initial_sump_liters.maximum > case.maximum_sump_liters
    {
        return Err(format!("{}: malformed initial sump bounds", case.case_id));
    }
    if case.envelope.minimum_flow_liters_per_period == 0
        || case.envelope.minimum_flow_liters_per_period > case.envelope.maximum_flow_liters_per_period
        || case.envelope.maximum_head_kpa == 0
        || case.envelope.minimum_on_periods == 0
    {
        return Err(format!("{}: malformed asset envelope", case.case_id));
    }
    for (index, period) in case.periods.iter().enumerate() {
        if period.inflow_liters.minimum > period.inflow_liters.maximum
            || period.required_head_kpa.minimum > period.required_head_kpa.maximum
        {
            return Err(format!("{}: malformed bounds at period {}", case.case_id, index));
        }
    }
    Ok(())
}

fn evaluate_case(case: &Case) -> Check<CaseResult> {
    validate_case_shape(case)?;

    let mut violations = Vec::new();
    let mut startup_count = 0_usize;
    let mut start_indices = Vec::new();
    let mut last_stop_index: Option<usize> = None;

    for (index, command_on) in case.commands_on.iter().copied().enumerate() {
        let was_on = index > 0 && case.commands_on[index - 1];
        if command_on && !was_on {
            startup_count += 1;
            start_indices.push(index);
            if let Some(stop_index) = last_stop_index {
                if index.saturating_sub(stop_index) < case.envelope.minimum_off_periods {
                    violations.push(violation(
                        "MINIMUM_OFF_TIME_NOT_MET",
                        Some(index),
                        Some(index.saturating_sub(stop_index) as i64),
                        Some(case.envelope.minimum_off_periods as i64),
                    ));
                }
            }
        }
        if !command_on && was_on {
            last_stop_index = Some(index);
        }
    }

    let mut run_start: Option<usize> = None;
    for index in 0..=case.commands_on.len() {
        let on = index < case.commands_on.len() && case.commands_on[index];
        match (run_start, on) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let run_length = index - start;
                if run_length < case.envelope.minimum_on_periods {
                    violations.push(violation(
                        "MINIMUM_ON_TIME_NOT_MET",
                        Some(start),
                        Some(run_length as i64),
                        Some(case.envelope.minimum_on_periods as i64),
                    ));
                }
                run_start = None;
            }
            _ => {}
        }
    }

    let mut state_min = i64::from(case.initial_sump_liters.minimum);
    let mut state_max = i64::from(case.initial_sump_liters.maximum);
    let mut trajectory = Vec::with_capacity(case.periods.len());

    for (index, period) in case.periods.iter().enumerate() {
        let command_on = case.commands_on[index];
        let start_index = start_indices.iter().copied().take_while(|start| *start <= index).last();
        let operating = command_on
            && start_index.is_some_and(|start| index.saturating_sub(start) >= case.envelope.startup_delay_periods);
        let started_this_period = command_on && (index == 0 || !case.commands_on[index - 1]);

        let flow = if operating {
            if period.required_head_kpa.maximum > case.envelope.maximum_head_kpa {
                violations.push(violation(
                    "HEAD_LIMIT_EXCEEDED",
                    Some(index),
                    Some(i64::from(period.required_head_kpa.maximum)),
                    Some(i64::from(case.envelope.maximum_head_kpa)),
                ));
            }
            Bounds {
                minimum: case.envelope.minimum_flow_liters_per_period,
                maximum: case.envelope.maximum_flow_liters_per_period,
            }
        } else {
            Bounds { minimum: 0, maximum: 0 }
        };

        let mut energy = if operating { case.envelope.operating_energy_wh_per_period } else { 0 };
        if started_this_period {
            energy = energy.checked_add(case.envelope.startup_energy_wh)
                .ok_or_else(|| format!("{}: startup energy overflow", case.case_id))?;
        }
        if energy > period.energy_budget_wh {
            violations.push(violation(
                "ENERGY_BUDGET_EXCEEDED",
                Some(index),
                Some(i64::from(energy)),
                Some(i64::from(period.energy_budget_wh)),
            ));
        }

        let next_min = state_min + i64::from(period.inflow_liters.minimum) - i64::from(flow.maximum);
        let next_max = state_max + i64::from(period.inflow_liters.maximum) - i64::from(flow.minimum);
        if next_min < i64::from(case.minimum_sump_liters) {
            violations.push(violation(
                "SUMP_LOW_BOUND_VIOLATION",
                Some(index),
                Some(next_min),
                Some(i64::from(case.minimum_sump_liters)),
            ));
        }
        if next_max > i64::from(case.maximum_sump_liters) {
            violations.push(violation(
                "SUMP_HIGH_BOUND_VIOLATION",
                Some(index),
                Some(next_max),
                Some(i64::from(case.maximum_sump_liters)),
            ));
        }

        let sump_after = SumpState { minimum_liters: next_min, maximum_liters: next_max };
        trajectory.push(PeriodResult {
            period_index: index,
            command_on,
            operating,
            inflow_liters: period.inflow_liters,
            pump_flow_liters_per_period: flow,
            sump_after: sump_after.clone(),
            energy_wh: energy,
            energy_budget_wh: period.energy_budget_wh,
        });
        state_min = next_min;
        state_max = next_max;
    }

    violations.sort_by(|left, right| {
        left.period_index.cmp(&right.period_index)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.observed.cmp(&right.observed))
    });
    Ok(CaseResult {
        case_id: case.case_id.clone(),
        expected_executable: case.expected_executable,
        executable: violations.is_empty(),
        startup_count,
        violations,
        trajectory,
    })
}

fn independent_conservation_mismatches(case: &Case, result: &CaseResult) -> usize {
    let mut prior_min = i64::from(case.initial_sump_liters.minimum);
    let mut prior_max = i64::from(case.initial_sump_liters.maximum);
    let mut mismatches = 0;
    for (period, output) in case.periods.iter().zip(&result.trajectory) {
        // This recomputation is intentionally separate from evaluate_case's state update.
        let reconstructed_min = prior_min + i64::from(period.inflow_liters.minimum)
            - i64::from(output.pump_flow_liters_per_period.maximum);
        let reconstructed_max = prior_max + i64::from(period.inflow_liters.maximum)
            - i64::from(output.pump_flow_liters_per_period.minimum);
        if reconstructed_min != output.sump_after.minimum_liters
            || reconstructed_max != output.sump_after.maximum_liters
        {
            mismatches += 1;
        }
        prior_min = output.sump_after.minimum_liters;
        prior_max = output.sump_after.maximum_liters;
    }
    mismatches
}

fn bounds(minimum: u32, maximum: u32) -> Bounds {
    Bounds { minimum, maximum }
}

fn period(inflow: Bounds, head: Bounds, budget: u32) -> PeriodInput {
    PeriodInput { inflow_liters: inflow, required_head_kpa: head, energy_budget_wh: budget }
}

fn default_envelope() -> Envelope {
    Envelope {
        minimum_flow_liters_per_period: 3,
        maximum_flow_liters_per_period: 6,
        maximum_head_kpa: 50,
        operating_energy_wh_per_period: 8,
        startup_energy_wh: 4,
        startup_delay_periods: 1,
        minimum_on_periods: 2,
        minimum_off_periods: 1,
    }
}

fn cases() -> Vec<Case> {
    let nominal_period = period(bounds(10, 12), bounds(30, 35), 20);
    let safe = Case {
        case_id: "steady-feasible".to_owned(),
        expected_executable: true,
        initial_sump_liters: bounds(20, 22),
        minimum_sump_liters: 5,
        maximum_sump_liters: 100,
        commands_on: vec![true, true, true],
        periods: vec![nominal_period.clone(), nominal_period.clone(), nominal_period.clone()],
        envelope: default_envelope(),
    };
    let low = Case {
        case_id: "pump-drawdown-low-level".to_owned(),
        expected_executable: false,
        initial_sump_liters: bounds(8, 10),
        minimum_sump_liters: 5,
        maximum_sump_liters: 50,
        commands_on: vec![true, true, true],
        periods: vec![period(bounds(0, 1), bounds(30, 35), 20); 3],
        envelope: default_envelope(),
    };
    let high = Case {
        case_id: "inflow-high-level".to_owned(),
        expected_executable: false,
        initial_sump_liters: bounds(40, 42),
        minimum_sump_liters: 5,
        maximum_sump_liters: 50,
        commands_on: vec![false, false, false],
        periods: vec![period(bounds(4, 6), bounds(30, 35), 20); 3],
        envelope: default_envelope(),
    };
    let mut high_head = Case {
        case_id: "pump-head-envelope".to_owned(),
        expected_executable: false,
        initial_sump_liters: bounds(20, 22),
        minimum_sump_liters: 5,
        maximum_sump_liters: 100,
        commands_on: vec![true, true, true],
        periods: vec![nominal_period.clone(), nominal_period.clone(), nominal_period.clone()],
        envelope: default_envelope(),
    };
    high_head.periods[1].required_head_kpa = bounds(60, 75);
    let min_on = Case {
        case_id: "minimum-on-time".to_owned(),
        expected_executable: false,
        initial_sump_liters: bounds(20, 22),
        minimum_sump_liters: 5,
        maximum_sump_liters: 100,
        commands_on: vec![true, false, false],
        periods: vec![nominal_period.clone(), nominal_period.clone(), nominal_period.clone()],
        envelope: default_envelope(),
    };
    let min_off = Case {
        case_id: "minimum-off-time".to_owned(),
        expected_executable: false,
        initial_sump_liters: bounds(20, 22),
        minimum_sump_liters: 5,
        maximum_sump_liters: 100,
        commands_on: vec![true, false, true],
        periods: vec![nominal_period.clone(), nominal_period.clone(), nominal_period.clone()],
        envelope: Envelope { minimum_off_periods: 2, minimum_on_periods: 1, ..default_envelope() },
    };
    let power = Case {
        case_id: "operating-energy-budget".to_owned(),
        expected_executable: false,
        initial_sump_liters: bounds(20, 22),
        minimum_sump_liters: 5,
        maximum_sump_liters: 100,
        commands_on: vec![true, true, true],
        periods: vec![period(bounds(10, 12), bounds(30, 35), 7); 3],
        envelope: default_envelope(),
    };
    vec![safe, low, high, high_head, min_on, min_off, power]
}

fn make_receipt(source_commit: &str) -> Check<Receipt> {
    if !is_lower_hex(source_commit, 40) {
        return Err("source commit must be a 40-character lowercase hexadecimal SHA".to_owned());
    }
    let inputs = cases();
    let mut outputs = Vec::with_capacity(inputs.len());
    let mut expected_controls_matched = 0;
    let mut accepted_cases = 0;
    let mut rejected_cases = 0;
    let mut hard_violations_in_accepted_cases = 0;
    let mut conservation_mismatches = 0;
    let mut period_count = 0;

    for input in &inputs {
        let output = evaluate_case(input)?;
        expected_controls_matched += usize::from(output.executable == output.expected_executable);
        accepted_cases += usize::from(output.executable);
        rejected_cases += usize::from(!output.executable);
        hard_violations_in_accepted_cases += usize::from(output.executable && !output.violations.is_empty());
        conservation_mismatches += independent_conservation_mismatches(input, &output);
        period_count += output.trajectory.len();
        outputs.push(output);
    }
    let case_bytes = serde_json::to_vec(&inputs)
        .map_err(|error| format!("cannot serialize generated case set: {error}"))?;
    let generated_case_set_sha256 = sha256(&case_bytes);
    if expected_controls_matched != inputs.len()
        || accepted_cases != 1
        || rejected_cases != inputs.len() - 1
        || hard_violations_in_accepted_cases != 0
        || conservation_mismatches != 0
    {
        return Err("pump/sump synthetic acceptance contract failed".to_owned());
    }
    Ok(Receipt {
        result: "validated".to_owned(),
        suite_id: SUITE_ID.to_owned(),
        suite_version: SUITE_VERSION.to_owned(),
        source_commit: source_commit.to_owned(),
        generated_case_set_sha256,
        case_count: inputs.len(),
        period_count,
        expected_controls_matched,
        accepted_cases,
        rejected_cases,
        hard_violations_in_accepted_cases,
        conservation_mismatches,
        cases: outputs,
        claim_ceiling: "finite synthetic pump/sump trajectory contract; not site-calibrated equipment capability or a safety case".to_owned(),
    })
}

pub(super) fn run_cli(args: &[String]) -> Check<ExitCode> {
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err("usage: mine-flex-scheduling-bench --pump-sump --source-commit <40-hex-sha>".to_owned());
    }
    let receipt = make_receipt(&args[1])?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize pump/sump receipt: {error}"))?
    );
    Ok(if receipt.result == "validated" { ExitCode::SUCCESS } else { ExitCode::from(2) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_pump_sump_contract_accepts_only_the_declared_safe_trace() {
        let receipt = make_receipt("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").expect("receipt");
        assert_eq!("validated", receipt.result);
        assert_eq!(7, receipt.case_count);
        assert_eq!(21, receipt.period_count);
        assert_eq!(1, receipt.accepted_cases);
        assert_eq!(6, receipt.rejected_cases);
        assert_eq!(7, receipt.expected_controls_matched);
        assert_eq!(0, receipt.hard_violations_in_accepted_cases);
        assert_eq!(0, receipt.conservation_mismatches);
        for case in &receipt.cases {
            assert_eq!(case.expected_executable, case.executable, "{}", case.case_id);
        }
    }

    #[test]
    fn head_minimum_duration_level_and_energy_controls_have_structured_failures() {
        let inputs = cases();
        let head = evaluate_case(&inputs[3]).expect("head control");
        assert!(head.violations.iter().any(|v| v.code == "HEAD_LIMIT_EXCEEDED"));
        let minimum_on = evaluate_case(&inputs[4]).expect("minimum-on control");
        assert!(minimum_on.violations.iter().any(|v| v.code == "MINIMUM_ON_TIME_NOT_MET"));
        let minimum_off = evaluate_case(&inputs[5]).expect("minimum-off control");
        assert!(minimum_off.violations.iter().any(|v| v.code == "MINIMUM_OFF_TIME_NOT_MET"));
        let power = evaluate_case(&inputs[6]).expect("energy control");
        assert!(power.violations.iter().any(|v| v.code == "ENERGY_BUDGET_EXCEEDED"));
        let low = evaluate_case(&inputs[1]).expect("low-level control");
        assert!(low.violations.iter().any(|v| v.code == "SUMP_LOW_BOUND_VIOLATION"));
        let high = evaluate_case(&inputs[2]).expect("high-level control");
        assert!(high.violations.iter().any(|v| v.code == "SUMP_HIGH_BOUND_VIOLATION"));
    }

    #[test]
    fn receipt_is_deterministic_and_rejects_invalid_source_identity() {
        let source = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        assert_eq!(
            serde_json::to_vec(&make_receipt(source).expect("first")).expect("serialize"),
            serde_json::to_vec(&make_receipt(source).expect("second")).expect("serialize")
        );
        assert!(make_receipt("not-a-sha").is_err());
    }

    #[test]
    fn malformed_time_grids_and_bounds_fail_closed() {
        let mut malformed = cases()[0].clone();
        malformed.commands_on.pop();
        assert!(evaluate_case(&malformed).is_err());
        let mut bad_bounds = cases()[0].clone();
        bad_bounds.periods[0].inflow_liters = bounds(10, 9);
        assert!(evaluate_case(&bad_bounds).is_err());
    }
}
