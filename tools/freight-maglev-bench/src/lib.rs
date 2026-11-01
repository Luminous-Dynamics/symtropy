//! Synthetic corridor corpus and independent hard-gate/lifecycle evaluator.
//!
//! This module compares declared candidate assumptions with an explicit
//! baseline. It is not a route optimizer or an engineering solver.

use std::collections::BTreeSet;

const CORPUS: &str = include_str!("../corpus/cases-v0.1.psv");
const DAYS_PER_YEAR: u64 = 365;
const EXPECTED_FIELDS: usize = 23;

/// Candidate freight mode family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeFamily {
    NoBuild,
    ExistingRailUpgrade,
    ConventionalTunnelRail,
    MagRailRetrofit,
    DedicatedUndergroundMaglev,
    AutomatedFreightPod,
    Conveyor,
    Pipeline,
    ElectricRoad,
    InlandWaterway,
}

impl ModeFamily {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "no_build" => Ok(Self::NoBuild),
            "existing_rail_upgrade" => Ok(Self::ExistingRailUpgrade),
            "conventional_tunnel_rail" => Ok(Self::ConventionalTunnelRail),
            "magrail_retrofit" => Ok(Self::MagRailRetrofit),
            "dedicated_underground_maglev" => Ok(Self::DedicatedUndergroundMaglev),
            "automated_freight_pod" => Ok(Self::AutomatedFreightPod),
            "conveyor" => Ok(Self::Conveyor),
            "pipeline" => Ok(Self::Pipeline),
            "electric_road" => Ok(Self::ElectricRoad),
            "inland_waterway" => Ok(Self::InlandWaterway),
            _ => Err(format!("unknown mode family: {value}")),
        }
    }
}

/// Cargo compatibility class used by the synthetic fixtures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CargoClass {
    Containers,
    Bulk,
    Liquid,
    Mixed,
}

impl CargoClass {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "containers" => Ok(Self::Containers),
            "bulk" => Ok(Self::Bulk),
            "liquid" => Ok(Self::Liquid),
            "mixed" => Ok(Self::Mixed),
            _ => Err(format!("unknown cargo class: {value}")),
        }
    }
}

/// Evidence state for a non-compensable gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Pass,
    Fail,
    Unknown,
}

impl Gate {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pass" => Ok(Self::Pass),
            "fail" => Ok(Self::Fail),
            "unknown" => Ok(Self::Unknown),
            _ => Err(format!("unknown gate state: {value}")),
        }
    }
}

/// Expected candidate disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Eligible,
    InsufficientCapacity,
    NeedsEvidence,
    Ineligible,
}

impl Disposition {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "eligible" => Ok(Self::Eligible),
            "insufficient_capacity" => Ok(Self::InsufficientCapacity),
            "needs_evidence" => Ok(Self::NeedsEvidence),
            "ineligible" => Ok(Self::Ineligible),
            _ => Err(format!("unknown expected disposition: {value}")),
        }
    }
}

/// One immutable synthetic comparison case.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CorridorCase {
    pub id: String,
    pub family: ModeFamily,
    pub cargo: CargoClass,
    pub demand_tonnes_per_day: u64,
    pub candidate_capacity_tonnes_per_day: u64,
    pub baseline_capacity_tonnes_per_day: u64,
    pub candidate_fixed_annual_cost_minor: u64,
    pub baseline_fixed_annual_cost_minor: u64,
    pub candidate_variable_cost_minor_per_tonne: u64,
    pub baseline_variable_cost_minor_per_tonne: u64,
    pub candidate_transit_minutes: u64,
    pub baseline_transit_minutes: u64,
    pub candidate_energy_wh_per_tonne: u64,
    pub baseline_energy_wh_per_tonne: u64,
    pub candidate_emissions_gco2e_per_tonne: u64,
    pub baseline_emissions_gco2e_per_tonne: u64,
    pub rights: Gate,
    pub geotechnical: Gate,
    pub fire_egress: Gate,
    pub environmental: Gate,
    pub cargo_compatibility: Gate,
    pub fault_capacity_remaining_percent: u64,
    pub expected: Disposition,
}

/// Deterministic result. Fault inputs are stress cases, not probabilities.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Evaluation {
    pub id: String,
    pub disposition: Disposition,
    pub candidate_delivered_tonnes_per_day: u64,
    pub candidate_unmet_tonnes_per_day: u64,
    pub baseline_delivered_tonnes_per_day: u64,
    pub baseline_unmet_tonnes_per_day: u64,
    pub candidate_annual_cost_minor: u64,
    pub baseline_annual_cost_minor: u64,
    pub candidate_annual_energy_wh: u64,
    pub baseline_annual_energy_wh: u64,
    pub candidate_annual_emissions_gco2e: u64,
    pub baseline_annual_emissions_gco2e: u64,
    pub fault_delivered_tonnes_per_day: u64,
    pub fault_unmet_tonnes_per_day: u64,
    pub break_even_demand_tonnes_per_day: Option<u64>,
}

/// Parse or arithmetic errors are explicit and fail the run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BenchError {
    Parse { line: usize, message: String },
    DuplicateId(String),
    InvalidCase { id: String, message: String },
    ArithmeticOverflow { id: String },
}

impl std::fmt::Display for BenchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse { line, message } => write!(formatter, "line {line}: {message}"),
            Self::DuplicateId(id) => write!(formatter, "duplicate case id: {id}"),
            Self::InvalidCase { id, message } => write!(formatter, "{id}: {message}"),
            Self::ArithmeticOverflow { id } => write!(formatter, "{id}: arithmetic overflow"),
        }
    }
}

impl std::error::Error for BenchError {}

fn parse_u64(value: &str, name: &str, line: usize) -> Result<u64, BenchError> {
    value.parse::<u64>().map_err(|_| BenchError::Parse {
        line,
        message: format!("{name} must be a non-negative integer"),
    })
}

fn parse_gate(value: &str, name: &str, line: usize) -> Result<Gate, BenchError> {
    Gate::parse(value).map_err(|message| BenchError::Parse {
        line,
        message: format!("{name}: {message}"),
    })
}

fn parse_enum<T>(result: Result<T, String>, name: &str, line: usize) -> Result<T, BenchError> {
    result.map_err(|message| BenchError::Parse {
        line,
        message: format!("{name}: {message}"),
    })
}

/// Strictly parse the 23-column frozen corpus. Unknown columns are rejected.
pub fn parse_corpus(input: &str) -> Result<Vec<CorridorCase>, BenchError> {
    let mut cases = Vec::new();
    let mut ids = BTreeSet::new();

    for (line_index, raw) in input.lines().enumerate() {
        let line_number = line_index + 1;
        let row = raw.trim();
        if row.is_empty() || row.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = row.split('|').collect();
        if fields.len() != EXPECTED_FIELDS {
            return Err(BenchError::Parse {
                line: line_number,
                message: format!("expected {EXPECTED_FIELDS} fields, found {}", fields.len()),
            });
        }
        let id = fields[0].to_owned();
        if id.is_empty() || !ids.insert(id.clone()) {
            return Err(BenchError::DuplicateId(id));
        }
        let case = CorridorCase {
            id,
            family: parse_enum(ModeFamily::parse(fields[1]), "family", line_number)?,
            cargo: parse_enum(CargoClass::parse(fields[2]), "cargo", line_number)?,
            demand_tonnes_per_day: parse_u64(fields[3], "demand", line_number)?,
            candidate_capacity_tonnes_per_day: parse_u64(fields[4], "candidate capacity", line_number)?,
            baseline_capacity_tonnes_per_day: parse_u64(fields[5], "baseline capacity", line_number)?,
            candidate_fixed_annual_cost_minor: parse_u64(fields[6], "candidate fixed cost", line_number)?,
            baseline_fixed_annual_cost_minor: parse_u64(fields[7], "baseline fixed cost", line_number)?,
            candidate_variable_cost_minor_per_tonne: parse_u64(fields[8], "candidate variable cost", line_number)?,
            baseline_variable_cost_minor_per_tonne: parse_u64(fields[9], "baseline variable cost", line_number)?,
            candidate_transit_minutes: parse_u64(fields[10], "candidate transit minutes", line_number)?,
            baseline_transit_minutes: parse_u64(fields[11], "baseline transit minutes", line_number)?,
            candidate_energy_wh_per_tonne: parse_u64(fields[12], "candidate energy intensity", line_number)?,
            baseline_energy_wh_per_tonne: parse_u64(fields[13], "baseline energy intensity", line_number)?,
            candidate_emissions_gco2e_per_tonne: parse_u64(fields[14], "candidate emissions intensity", line_number)?,
            baseline_emissions_gco2e_per_tonne: parse_u64(fields[15], "baseline emissions intensity", line_number)?,
            rights: parse_gate(fields[16], "rights", line_number)?,
            geotechnical: parse_gate(fields[17], "geotechnical", line_number)?,
            fire_egress: parse_gate(fields[18], "fire/egress", line_number)?,
            environmental: parse_gate(fields[19], "environmental", line_number)?,
            cargo_compatibility: parse_gate(fields[20], "cargo compatibility", line_number)?,
            fault_capacity_remaining_percent: parse_u64(fields[21], "fault capacity percent", line_number)?,
            expected: parse_enum(Disposition::parse(fields[22]), "expected disposition", line_number)?,
        };

        if case.demand_tonnes_per_day == 0
            || case.candidate_capacity_tonnes_per_day == 0
            || case.baseline_capacity_tonnes_per_day == 0
        {
            return Err(BenchError::InvalidCase {
                id: case.id,
                message: "demand and both capacities must be positive".to_owned(),
            });
        }
        if case.fault_capacity_remaining_percent > 100 {
            return Err(BenchError::InvalidCase {
                id: case.id,
                message: "fault capacity percent must be between 0 and 100".to_owned(),
            });
        }
        cases.push(case);
    }

    Ok(cases)
}

/// Apply hard gates before capacity. Costs and benefits cannot compensate for
/// failed or unknown rights, geology, fire/egress, environmental or cargo gates.
#[must_use]
pub fn disposition(case: &CorridorCase) -> Disposition {
    let gates = [
        case.rights,
        case.geotechnical,
        case.fire_egress,
        case.environmental,
        case.cargo_compatibility,
    ];
    if gates.contains(&Gate::Fail) {
        Disposition::Ineligible
    } else if gates.contains(&Gate::Unknown) {
        Disposition::NeedsEvidence
    } else if case.candidate_capacity_tonnes_per_day < case.demand_tonnes_per_day {
        Disposition::InsufficientCapacity
    } else {
        Disposition::Eligible
    }
}

fn annual_cost(fixed: u64, variable_per_tonne: u64, delivered_tonnes_per_day: u64, id: &str) -> Result<u64, BenchError> {
    let annual_delivered = delivered_tonnes_per_day
        .checked_mul(DAYS_PER_YEAR)
        .ok_or_else(|| BenchError::ArithmeticOverflow { id: id.to_owned() })?;
    let variable = annual_delivered
        .checked_mul(variable_per_tonne)
        .ok_or_else(|| BenchError::ArithmeticOverflow { id: id.to_owned() })?;
    fixed.checked_add(variable)
        .ok_or_else(|| BenchError::ArithmeticOverflow { id: id.to_owned() })
}

fn annual_intensity(per_tonne: u64, delivered_tonnes_per_day: u64, id: &str) -> Result<u64, BenchError> {
    delivered_tonnes_per_day
        .checked_mul(DAYS_PER_YEAR)
        .and_then(|annual| annual.checked_mul(per_tonne))
        .ok_or_else(|| BenchError::ArithmeticOverflow { id: id.to_owned() })
}

fn break_even_demand_tonnes_per_day(case: &CorridorCase) -> Option<u64> {
    let fixed_delta = case.candidate_fixed_annual_cost_minor
        .saturating_sub(case.baseline_fixed_annual_cost_minor);
    let variable_saving = case.baseline_variable_cost_minor_per_tonne
        .checked_sub(case.candidate_variable_cost_minor_per_tonne)?;
    if fixed_delta == 0 {
        return Some(0);
    }
    if variable_saving == 0 {
        return None;
    }
    let annual = fixed_delta
        .checked_add(variable_saving - 1)?
        .checked_div(variable_saving)?;
    annual.checked_add(DAYS_PER_YEAR - 1).map(|n| n / DAYS_PER_YEAR)
}

/// Evaluate a single candidate against the explicit baseline.
pub fn evaluate(case: &CorridorCase) -> Result<Evaluation, BenchError> {
    let candidate_delivered = case.demand_tonnes_per_day.min(case.candidate_capacity_tonnes_per_day);
    let baseline_delivered = case.demand_tonnes_per_day.min(case.baseline_capacity_tonnes_per_day);
    let fault_capacity = case.candidate_capacity_tonnes_per_day
        .checked_mul(case.fault_capacity_remaining_percent)
        .ok_or_else(|| BenchError::ArithmeticOverflow { id: case.id.clone() })? / 100;
    let fault_delivered = case.demand_tonnes_per_day.min(fault_capacity);

    Ok(Evaluation {
        id: case.id.clone(),
        disposition: disposition(case),
        candidate_delivered_tonnes_per_day: candidate_delivered,
        candidate_unmet_tonnes_per_day: case.demand_tonnes_per_day - candidate_delivered,
        baseline_delivered_tonnes_per_day: baseline_delivered,
        baseline_unmet_tonnes_per_day: case.demand_tonnes_per_day - baseline_delivered,
        candidate_annual_cost_minor: annual_cost(
            case.candidate_fixed_annual_cost_minor,
            case.candidate_variable_cost_minor_per_tonne,
            candidate_delivered,
            &case.id,
        )?,
        baseline_annual_cost_minor: annual_cost(
            case.baseline_fixed_annual_cost_minor,
            case.baseline_variable_cost_minor_per_tonne,
            baseline_delivered,
            &case.id,
        )?,
        candidate_annual_energy_wh: annual_intensity(
            case.candidate_energy_wh_per_tonne,
            candidate_delivered,
            &case.id,
        )?,
        baseline_annual_energy_wh: annual_intensity(
            case.baseline_energy_wh_per_tonne,
            baseline_delivered,
            &case.id,
        )?,
        candidate_annual_emissions_gco2e: annual_intensity(
            case.candidate_emissions_gco2e_per_tonne,
            candidate_delivered,
            &case.id,
        )?,
        baseline_annual_emissions_gco2e: annual_intensity(
            case.baseline_emissions_gco2e_per_tonne,
            baseline_delivered,
            &case.id,
        )?,
        fault_delivered_tonnes_per_day: fault_delivered,
        fault_unmet_tonnes_per_day: case.demand_tonnes_per_day - fault_delivered,
        break_even_demand_tonnes_per_day: break_even_demand_tonnes_per_day(case),
    })
}

/// Parse and evaluate the checked-in frozen v0.1 corpus.
pub fn evaluate_frozen_corpus() -> Result<Vec<Evaluation>, BenchError> {
    let cases = parse_corpus(CORPUS)?;
    cases.iter().map(evaluate).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_corpus_contains_24_unique_cases_and_all_expected_modes() {
        let cases = parse_corpus(CORPUS).unwrap();
        assert_eq!(cases.len(), 25);
        let modes: BTreeSet<_> = cases.iter().map(|case| format!("{:?}", case.family)).collect();
        assert_eq!(modes.len(), 10);
    }

    #[test]
    fn every_frozen_case_matches_its_expected_disposition() {
        for case in parse_corpus(CORPUS).unwrap() {
            assert_eq!(disposition(&case), case.expected, "case {}", case.id);
        }
    }

    #[test]
    fn failed_hard_gate_precedes_cost_or_capacity() {
        let cases = parse_corpus(CORPUS).unwrap();
        let case = cases.iter().find(|item| item.id == "pod_fire_egress_fail").unwrap();
        let result = evaluate(case).unwrap();
        assert_eq!(result.disposition, Disposition::Ineligible);
        assert!(result.candidate_annual_cost_minor < result.baseline_annual_cost_minor);
    }

    #[test]
    fn unknown_rights_are_needs_evidence_not_eligible() {
        let cases = parse_corpus(CORPUS).unwrap();
        let case = cases.iter().find(|item| item.id == "magrail_rights_unknown").unwrap();
        assert_eq!(disposition(case), Disposition::NeedsEvidence);
    }

    #[test]
    fn exact_capacity_meets_declared_demand() {
        let cases = parse_corpus(CORPUS).unwrap();
        let case = cases.iter().find(|item| item.id == "capacity_exactly_demand").unwrap();
        let result = evaluate(case).unwrap();
        assert_eq!(result.disposition, Disposition::Eligible);
        assert_eq!(result.candidate_unmet_tonnes_per_day, 0);
    }

    #[test]
    fn deterministic_fault_is_not_reported_as_a_probability() {
        let cases = parse_corpus(CORPUS).unwrap();
        let case = cases.iter().find(|item| item.id == "magrail_portal_outage").unwrap();
        let result = evaluate(case).unwrap();
        assert_eq!(result.fault_delivered_tonnes_per_day, 0);
        assert_eq!(result.fault_unmet_tonnes_per_day, case.demand_tonnes_per_day);
    }

    #[test]
    fn parser_rejects_unknown_extra_columns() {
        let result = parse_corpus("id|extra\n");
        assert!(matches!(result, Err(BenchError::Parse { .. })));
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let row = "duplicate|no_build|mixed|1|1|1|0|0|1|1|1|1|1|1|1|1|pass|pass|pass|pass|pass|100|eligible";
        let corpus = format!("{row}\n{row}\n");
        assert_eq!(parse_corpus(&corpus), Err(BenchError::DuplicateId("duplicate".to_owned())));
    }
}
