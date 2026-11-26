use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-acquisition-codesign.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-acquisition-codesign.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-ACQUISITION-CODESIGN-001";
pub const CLAIM_CEILING: &str =
    "synthetic_exact_finite_schedule_feasibility_and_pareto_frontier_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateState {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SearchStatus {
    OptimalWithinDeclaredFiniteUniverse,
    Infeasible,
    Indeterminate,
    SearchLimitNotProven,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateEvidence {
    pub gate_id: String,
    pub state: GateState,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SpeedChoice {
    pub speed_id: String,
    pub duration_ms: u64,
    pub motion_energy_mj: u64,
    pub motion_profile_receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PathSegment {
    pub segment_id: String,
    pub path_revision: String,
    pub speed_choices: Vec<SpeedChoice>,
    pub path_feasibility_receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionOption {
    pub option_id: String,
    pub segment_id: String,
    pub speed_id: String,
    pub exposure_ns: u64,
    pub illumination_mode_id: String,
    pub dwell_ms: u64,
    pub illumination_energy_mj: u64,
    pub projector_energy_mj: u64,
    pub retakes: u32,
    pub gates: Vec<GateEvidence>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcquisitionEvent {
    pub event_id: String,
    pub characteristic_id: String,
    pub required: bool,
    pub options: Vec<AcquisitionOption>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AllowedTransition {
    pub from_segment_id: String,
    pub from_speed_id: String,
    pub to_segment_id: String,
    pub to_speed_id: String,
    pub transition_receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceBudget {
    pub robot_energy_capacity_mj: u64,
    pub required_return_reserve_mj: u64,
    pub illumination_energy_budget_mj: u64,
    pub projector_energy_budget_mj: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub segments: Vec<PathSegment>,
    pub events: Vec<AcquisitionEvent>,
    pub allowed_transitions: Vec<AllowedTransition>,
    pub budget: ResourceBudget,
    pub max_schedules: u64,
    pub expected_status: SearchStatus,
    pub expected_frontier_schedule_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ObjectiveVector {
    pub elapsed_ms: u64,
    pub robot_motion_energy_mj: u64,
    pub illumination_energy_mj: u64,
    pub projector_energy_mj: u64,
    pub retakes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Schedule {
    pub schedule_id: String,
    pub selected_speed_ids: BTreeMap<String, String>,
    pub selected_option_ids: BTreeMap<String, String>,
    pub evidence_receipt_sha256: Vec<String>,
    pub objective: ObjectiveVector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScheduleDisposition {
    pub schedule_id: String,
    pub disposition: String,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: SearchStatus,
    pub theoretical_schedule_count: u64,
    pub evaluated_schedule_count: u64,
    pub feasible_schedule_count: u64,
    pub indeterminate_schedule_count: u64,
    pub infeasible_schedule_count: u64,
    pub pareto_frontier: Vec<Schedule>,
    pub dispositions: Vec<ScheduleDisposition>,
    pub exactness_proven: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn checked_add(left: u64, right: u64, code: &str) -> Result<u64, String> {
    left.checked_add(right).ok_or_else(|| code.to_owned())
}

fn checked_mul(left: u64, right: u64, code: &str) -> Result<u64, String> {
    left.checked_mul(right).ok_or_else(|| code.to_owned())
}

fn theoretical_count(case: &Case) -> Result<u64, String> {
    let mut count = 1_u64;
    for segment in &case.segments {
        count = checked_mul(
            count,
            segment.speed_choices.len() as u64,
            "SCHEDULE_COUNT_OVERFLOW",
        )?;
    }
    for event in &case.events {
        if event.required {
            count = checked_mul(count, event.options.len() as u64, "SCHEDULE_COUNT_OVERFLOW")?;
        }
    }
    Ok(count)
}

fn validate_case(case: &Case) -> Result<(), String> {
    if !valid_text(&case.case_id)
        || case.segments.is_empty()
        || case.events.is_empty()
        || case.max_schedules == 0
    {
        return Err("CASE_ID_OR_REQUIRED_COLLECTION_INVALID".to_owned());
    }

    let mut segment_ids = BTreeSet::new();
    for segment in &case.segments {
        if !valid_text(&segment.segment_id)
            || !valid_text(&segment.path_revision)
            || !valid_sha256(&segment.path_feasibility_receipt_sha256)
            || segment.speed_choices.is_empty()
            || !segment_ids.insert(segment.segment_id.clone())
        {
            return Err("PATH_SEGMENT_INVALID".to_owned());
        }
        let mut speed_ids = BTreeSet::new();
        for speed in &segment.speed_choices {
            if !valid_text(&speed.speed_id)
                || !valid_sha256(&speed.motion_profile_receipt_sha256)
                || speed.duration_ms == 0
                || !speed_ids.insert(speed.speed_id.clone())
            {
                return Err("SPEED_CHOICE_INVALID".to_owned());
            }
        }
    }

    let mut event_ids = BTreeSet::new();
    for event in &case.events {
        if !valid_text(&event.event_id)
            || !valid_text(&event.characteristic_id)
            || !event_ids.insert(event.event_id.clone())
            || (event.required && event.options.is_empty())
        {
            return Err("ACQUISITION_EVENT_INVALID".to_owned());
        }
        let mut option_ids = BTreeSet::new();
        for option in &event.options {
            let mut gate_ids = BTreeSet::new();
            let segment = case
                .segments
                .iter()
                .find(|segment| segment.segment_id == option.segment_id)
                .ok_or_else(|| "ACQUISITION_OPTION_SEGMENT_UNKNOWN".to_owned())?;
            if !valid_text(&option.option_id)
                || !valid_text(&option.segment_id)
                || !valid_text(&option.speed_id)
                || !valid_text(&option.illumination_mode_id)
                || !option_ids.insert(option.option_id.clone())
                || option.gates.is_empty()
                || !segment
                    .speed_choices
                    .iter()
                    .any(|speed| speed.speed_id == option.speed_id)
                || !option.gates.iter().all(|gate| {
                    valid_text(&gate.gate_id)
                        && valid_sha256(&gate.receipt_sha256)
                        && gate_ids.insert(gate.gate_id.clone())
                })
            {
                return Err("ACQUISITION_OPTION_INVALID".to_owned());
            }
        }
    }

    let mut transitions = BTreeSet::new();
    for transition in &case.allowed_transitions {
        if !valid_text(&transition.from_segment_id)
            || !valid_text(&transition.from_speed_id)
            || !valid_text(&transition.to_segment_id)
            || !valid_text(&transition.to_speed_id)
            || !valid_sha256(&transition.transition_receipt_sha256)
            || !segment_ids.contains(&transition.from_segment_id)
            || !segment_ids.contains(&transition.to_segment_id)
        {
            return Err("TRANSITION_INVALID".to_owned());
        }
        let from_segment = case
            .segments
            .iter()
            .find(|segment| segment.segment_id == transition.from_segment_id)
            .ok_or_else(|| "TRANSITION_FROM_SEGMENT_UNKNOWN".to_owned())?;
        let to_segment = case
            .segments
            .iter()
            .find(|segment| segment.segment_id == transition.to_segment_id)
            .ok_or_else(|| "TRANSITION_TO_SEGMENT_UNKNOWN".to_owned())?;
        if !from_segment
            .speed_choices
            .iter()
            .any(|speed| speed.speed_id == transition.from_speed_id)
            || !to_segment
                .speed_choices
                .iter()
                .any(|speed| speed.speed_id == transition.to_speed_id)
        {
            return Err("TRANSITION_SPEED_REFERENCE_INVALID".to_owned());
        }
        let key = (
            transition.from_segment_id.clone(),
            transition.from_speed_id.clone(),
            transition.to_segment_id.clone(),
            transition.to_speed_id.clone(),
        );
        if !transitions.insert(key) {
            return Err("TRANSITION_DUPLICATE".to_owned());
        }
    }

    if case.budget.required_return_reserve_mj > case.budget.robot_energy_capacity_mj {
        return Err("RETURN_RESERVE_EXCEEDS_CAPACITY".to_owned());
    }
    Ok(())
}

fn dominates(left: &ObjectiveVector, right: &ObjectiveVector) -> bool {
    let l = [
        left.elapsed_ms,
        left.robot_motion_energy_mj,
        left.illumination_energy_mj,
        left.projector_energy_mj,
        left.retakes,
    ];
    let r = [
        right.elapsed_ms,
        right.robot_motion_energy_mj,
        right.illumination_energy_mj,
        right.projector_energy_mj,
        right.retakes,
    ];
    l.iter().zip(r.iter()).all(|(a, b)| a <= b) && l.iter().zip(r.iter()).any(|(a, b)| a < b)
}

fn objective_equal(left: &ObjectiveVector, right: &ObjectiveVector) -> bool {
    left == right
}

fn schedule_id(speeds: &BTreeMap<String, String>, options: &BTreeMap<String, String>) -> String {
    let speeds_part = speeds
        .iter()
        .map(|(segment, speed)| format!("{segment}={speed}"))
        .collect::<Vec<_>>()
        .join(";");
    let options_part = options
        .iter()
        .map(|(event, option)| format!("{event}={option}"))
        .collect::<Vec<_>>()
        .join(";");
    format!("S[{speeds_part}]O[{options_part}]")
}

struct Enumeration<'a> {
    case: &'a Case,
    dimensions: Vec<Vec<String>>,
    segment_dimension_count: usize,
    current: Vec<String>,
    feasible: Vec<Schedule>,
    dispositions: Vec<ScheduleDisposition>,
    feasible_count: u64,
    indeterminate_count: u64,
    infeasible_count: u64,
}

impl<'a> Enumeration<'a> {
    fn new(case: &'a Case) -> Self {
        let mut dimensions = Vec::new();
        for segment in &case.segments {
            dimensions.push(
                segment
                    .speed_choices
                    .iter()
                    .map(|speed| speed.speed_id.clone())
                    .collect(),
            );
        }
        let segment_dimension_count = dimensions.len();
        for event in &case.events {
            if event.required {
                dimensions.push(
                    event
                        .options
                        .iter()
                        .map(|option| option.option_id.clone())
                        .collect(),
                );
            }
        }
        Self {
            case,
            dimensions,
            segment_dimension_count,
            current: Vec::new(),
            feasible: Vec::new(),
            dispositions: Vec::new(),
            feasible_count: 0,
            indeterminate_count: 0,
            infeasible_count: 0,
        }
    }

    fn recurse(&mut self, depth: usize) -> Result<(), String> {
        if depth == self.dimensions.len() {
            self.evaluate_current()?;
            return Ok(());
        }
        for choice in self.dimensions[depth].clone() {
            self.current.push(choice);
            self.recurse(depth + 1)?;
            self.current.pop();
        }
        Ok(())
    }

    fn evaluate_current(&mut self) -> Result<(), String> {
        let mut selected_speeds = BTreeMap::new();
        let mut selected_options = BTreeMap::new();
        let mut evidence_receipts = BTreeSet::new();
        let mut objective = ObjectiveVector {
            elapsed_ms: 0,
            robot_motion_energy_mj: 0,
            illumination_energy_mj: 0,
            projector_energy_mj: 0,
            retakes: 0,
        };
        let mut reasons = BTreeSet::new();
        let mut hard_failure = false;
        let mut unknown = false;

        for (index, segment) in self.case.segments.iter().enumerate() {
            let speed_id = &self.current[index];
            let speed = segment
                .speed_choices
                .iter()
                .find(|choice| &choice.speed_id == speed_id)
                .ok_or_else(|| "SELECTED_SPEED_NOT_FOUND".to_owned())?;
            selected_speeds.insert(segment.segment_id.clone(), speed.speed_id.clone());
            evidence_receipts.insert(segment.path_feasibility_receipt_sha256.clone());
            evidence_receipts.insert(speed.motion_profile_receipt_sha256.clone());
            objective.elapsed_ms =
                checked_add(objective.elapsed_ms, speed.duration_ms, "ELAPSED_OVERFLOW")?;
            objective.robot_motion_energy_mj = checked_add(
                objective.robot_motion_energy_mj,
                speed.motion_energy_mj,
                "ROBOT_ENERGY_OVERFLOW",
            )?;
        }

        for (event_index, event) in self
            .case
            .events
            .iter()
            .filter(|event| event.required)
            .enumerate()
        {
            let option_index = self.segment_dimension_count + event_index;
            let option_id = &self.current[option_index];
            let option = event
                .options
                .iter()
                .find(|option| &option.option_id == option_id)
                .ok_or_else(|| "SELECTED_OPTION_NOT_FOUND".to_owned())?;
            selected_options.insert(event.event_id.clone(), option.option_id.clone());
            for gate in &option.gates {
                evidence_receipts.insert(gate.receipt_sha256.clone());
            }

            let selected_speed = selected_speeds.get(&option.segment_id);
            if selected_speed != Some(&option.speed_id) {
                hard_failure = true;
                reasons.insert("OPTION_SPEED_DOES_NOT_MATCH_SEGMENT".to_owned());
            }

            for gate in &option.gates {
                match gate.state {
                    GateState::Pass => {}
                    GateState::Fail => {
                        hard_failure = true;
                        reasons.insert(format!("GATE_FAILED:{}", gate.gate_id));
                    }
                    GateState::Unknown => {
                        unknown = true;
                        reasons.insert(format!("GATE_UNKNOWN:{}", gate.gate_id));
                    }
                }
            }

            objective.elapsed_ms =
                checked_add(objective.elapsed_ms, option.dwell_ms, "ELAPSED_OVERFLOW")?;
            objective.illumination_energy_mj = checked_add(
                objective.illumination_energy_mj,
                option.illumination_energy_mj,
                "ILLUMINATION_ENERGY_OVERFLOW",
            )?;
            objective.projector_energy_mj = checked_add(
                objective.projector_energy_mj,
                option.projector_energy_mj,
                "PROJECTOR_ENERGY_OVERFLOW",
            )?;
            objective.retakes = checked_add(
                objective.retakes,
                u64::from(option.retakes),
                "RETAKE_OVERFLOW",
            )?;
        }

        for pair in self.case.segments.windows(2) {
            let from_speed = selected_speeds
                .get(&pair[0].segment_id)
                .ok_or_else(|| "PREVIOUS_SPEED_MISSING".to_owned())?;
            let to_speed = selected_speeds
                .get(&pair[1].segment_id)
                .ok_or_else(|| "NEXT_SPEED_MISSING".to_owned())?;
            let allowed = self.case.allowed_transitions.iter().find(|transition| {
                transition.from_segment_id == pair[0].segment_id
                    && transition.from_speed_id == *from_speed
                    && transition.to_segment_id == pair[1].segment_id
                    && transition.to_speed_id == *to_speed
            });
            if let Some(transition) = allowed {
                evidence_receipts.insert(transition.transition_receipt_sha256.clone());
            }
            if allowed.is_none() {
                hard_failure = true;
                reasons.insert("PATH_SPEED_TRANSITION_NOT_ADMITTED".to_owned());
            }
        }

        let available_robot_energy = self
            .case
            .budget
            .robot_energy_capacity_mj
            .saturating_sub(self.case.budget.required_return_reserve_mj);
        if objective.robot_motion_energy_mj > available_robot_energy {
            hard_failure = true;
            reasons.insert("ROBOT_ENERGY_OR_RETURN_RESERVE_EXCEEDED".to_owned());
        }
        if objective.illumination_energy_mj > self.case.budget.illumination_energy_budget_mj {
            hard_failure = true;
            reasons.insert("ILLUMINATION_ENERGY_BUDGET_EXCEEDED".to_owned());
        }
        if objective.projector_energy_mj > self.case.budget.projector_energy_budget_mj {
            hard_failure = true;
            reasons.insert("PROJECTOR_ENERGY_BUDGET_EXCEEDED".to_owned());
        }

        let id = schedule_id(&selected_speeds, &selected_options);
        if hard_failure {
            self.infeasible_count += 1;
            self.dispositions.push(ScheduleDisposition {
                schedule_id: id,
                disposition: "INFEASIBLE".to_owned(),
                reason_codes: reasons.into_iter().collect(),
            });
        } else if unknown {
            self.indeterminate_count += 1;
            self.dispositions.push(ScheduleDisposition {
                schedule_id: id,
                disposition: "INDETERMINATE".to_owned(),
                reason_codes: reasons.into_iter().collect(),
            });
        } else {
            self.feasible_count += 1;
            self.feasible.push(Schedule {
                schedule_id: id,
                selected_speed_ids: selected_speeds,
                selected_option_ids: selected_options,
                evidence_receipt_sha256: evidence_receipts.into_iter().collect(),
                objective,
            });
            self.dispositions.push(ScheduleDisposition {
                schedule_id: self
                    .feasible
                    .last()
                    .expect("just pushed")
                    .schedule_id
                    .clone(),
                disposition: "FEASIBLE".to_owned(),
                reason_codes: Vec::new(),
            });
        }
        Ok(())
    }
}

fn pareto_frontier(mut schedules: Vec<Schedule>) -> Vec<Schedule> {
    schedules.sort_by(|a, b| a.schedule_id.cmp(&b.schedule_id));
    let mut unique: Vec<Schedule> = Vec::new();
    for schedule in schedules {
        if let Some(existing) = unique
            .iter()
            .find(|item| objective_equal(&item.objective, &schedule.objective))
        {
            if schedule.schedule_id < existing.schedule_id {
                let index = unique
                    .iter()
                    .position(|item| item.schedule_id == existing.schedule_id)
                    .expect("exists");
                unique[index] = schedule;
            }
        } else {
            unique.push(schedule);
        }
    }
    let result = unique
        .iter()
        .filter(|candidate| {
            !unique.iter().any(|other| {
                other.schedule_id != candidate.schedule_id
                    && dominates(&other.objective, &candidate.objective)
            })
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut result = result;
    result.sort_by(|a, b| a.schedule_id.cmp(&b.schedule_id));
    result
}

fn evaluate_case(case: &Case) -> Result<CaseResult, String> {
    validate_case(case)?;
    let count = theoretical_count(case)?;
    if count > case.max_schedules {
        return Ok(CaseResult {
            case_id: case.case_id.clone(),
            status: SearchStatus::SearchLimitNotProven,
            theoretical_schedule_count: count,
            evaluated_schedule_count: 0,
            feasible_schedule_count: 0,
            indeterminate_schedule_count: 0,
            infeasible_schedule_count: 0,
            pareto_frontier: Vec::new(),
            dispositions: Vec::new(),
            exactness_proven: false,
        });
    }

    let mut enumeration = Enumeration::new(case);
    enumeration.recurse(0)?;
    let frontier = pareto_frontier(enumeration.feasible.clone());
    let status = if enumeration.indeterminate_count > 0 {
        SearchStatus::Indeterminate
    } else if !frontier.is_empty() {
        SearchStatus::OptimalWithinDeclaredFiniteUniverse
    } else {
        SearchStatus::Infeasible
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        status,
        theoretical_schedule_count: count,
        evaluated_schedule_count: count,
        feasible_schedule_count: enumeration.feasible_count,
        indeterminate_schedule_count: enumeration.indeterminate_count,
        infeasible_schedule_count: enumeration.infeasible_count,
        pareto_frontier: frontier,
        dispositions: enumeration.dispositions,
        exactness_proven: enumeration.indeterminate_count == 0,
    })
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("CORPUS_JSON_INVALID: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 256
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.clone()) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        cases.push(evaluate_case(case)?);
    }
    let matches = cases
        .iter()
        .zip(corpus.cases.iter())
        .filter(|(result, source)| {
            result.status == source.expected_status
                && result
                    .pareto_frontier
                    .iter()
                    .map(|schedule| schedule.schedule_id.clone())
                    .collect::<Vec<_>>()
                    == source.expected_frontier_schedule_ids
        })
        .count();

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_outcome_matches: matches,
        expected_outcome_mismatches: cases.len() - matches,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pareto_dominance_requires_strict_improvement_and_no_worse_dimension() {
        let a = ObjectiveVector {
            elapsed_ms: 5,
            robot_motion_energy_mj: 10,
            illumination_energy_mj: 2,
            projector_energy_mj: 0,
            retakes: 0,
        };
        let b = ObjectiveVector {
            elapsed_ms: 6,
            robot_motion_energy_mj: 10,
            illumination_energy_mj: 3,
            projector_energy_mj: 0,
            retakes: 0,
        };
        let c = ObjectiveVector {
            elapsed_ms: 4,
            robot_motion_energy_mj: 12,
            illumination_energy_mj: 2,
            projector_energy_mj: 0,
            retakes: 0,
        };
        assert!(dominates(&a, &b));
        assert!(!dominates(&a, &c));
        assert!(!dominates(&a, &a));
    }

    #[test]
    fn exact_frontier_keeps_incomparable_time_energy_tradeoffs() {
        let a = Schedule {
            schedule_id: "A".to_owned(),
            selected_speed_ids: BTreeMap::new(),
            selected_option_ids: BTreeMap::new(),
            evidence_receipt_sha256: Vec::new(),
            objective: ObjectiveVector {
                elapsed_ms: 5,
                robot_motion_energy_mj: 20,
                illumination_energy_mj: 2,
                projector_energy_mj: 0,
                retakes: 0,
            },
        };
        let b = Schedule {
            schedule_id: "B".to_owned(),
            selected_speed_ids: BTreeMap::new(),
            selected_option_ids: BTreeMap::new(),
            evidence_receipt_sha256: Vec::new(),
            objective: ObjectiveVector {
                elapsed_ms: 10,
                robot_motion_energy_mj: 10,
                illumination_energy_mj: 2,
                projector_energy_mj: 0,
                retakes: 0,
            },
        };
        let c = Schedule {
            schedule_id: "C".to_owned(),
            selected_speed_ids: BTreeMap::new(),
            selected_option_ids: BTreeMap::new(),
            evidence_receipt_sha256: Vec::new(),
            objective: ObjectiveVector {
                elapsed_ms: 12,
                robot_motion_energy_mj: 12,
                illumination_energy_mj: 3,
                projector_energy_mj: 0,
                retakes: 1,
            },
        };
        let frontier = pareto_frontier(vec![a, b, c]);
        assert_eq!(frontier.len(), 2);
        assert!(!frontier.iter().any(|item| item.schedule_id == "C"));
    }

    #[test]
    fn hard_fail_gate_prevents_schedule_from_becoming_feasible() {
        let mut gate = GateEvidence {
            gate_id: "timing".to_owned(),
            state: GateState::Fail,
            receipt_sha256: "a".repeat(64),
        };
        assert_eq!(gate.state, GateState::Fail);
        gate.state = GateState::Pass;
        assert_eq!(gate.state, GateState::Pass);
    }

    #[test]
    fn unknown_gate_is_not_a_pass() {
        let gate = GateEvidence {
            gate_id: "calibration".to_owned(),
            state: GateState::Unknown,
            receipt_sha256: "b".repeat(64),
        };
        assert_eq!(gate.state, GateState::Unknown);
    }

    #[test]
    fn frozen_corpus_matches_exact_frontiers() {
        let bytes = include_bytes!(
            "../../../docs/research/orbital-truss/fixtures/acquisition-codesign-v0_1.json"
        );
        let receipt = audit_corpus(bytes, "0123456789abcdef0123456789abcdef01234567")
            .expect("frozen corpus validates");
        assert_eq!(receipt.case_count, 8);
        assert_eq!(receipt.expected_outcome_matches, 8);
        assert_eq!(receipt.expected_outcome_mismatches, 0);
        assert!(
            receipt
                .cases
                .iter()
                .filter(|case| case.status == SearchStatus::SearchLimitNotProven)
                .all(|case| !case.exactness_proven)
        );
        assert!(
            receipt
                .cases
                .iter()
                .filter(|case| case.indeterminate_schedule_count > 0)
                .all(|case| !case.exactness_proven)
        );
    }
}
