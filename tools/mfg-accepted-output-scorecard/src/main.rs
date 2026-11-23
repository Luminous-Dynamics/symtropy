use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

const CASES_JSON: &str =
    include_str!("../../../fixtures/manufacturing/mfg_accepted_output_scorecard_cases_v0_1.json");
const ANSWERS_JSON: &str = include_str!(
    "../../../fixtures/manufacturing/mfg_accepted_output_scorecard_known_answers_v0_1.json"
);
const CASES_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_accepted_output_scorecard_cases_v0_1.schema.json"
);
const ANSWERS_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_accepted_output_scorecard_known_answers_v0_1.schema.json"
);
const PPM_SCALE: u128 = 1_000_000;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    #[serde(rename = "$schema")]
    json_schema: String,
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    case_count: usize,
    cases: Vec<EpisodeCase>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct EpisodeCase {
    case_id: String,
    scenario_family_id: String,
    disturbance_at_ms: u64,
    horizon_end_ms: u64,
    demand_units: u64,
    recovery_target_units: u64,
    outputs: Vec<OutputRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputRecord {
    output_id: String,
    produced_at_ms: u64,
    unit_ids: Vec<String>,
    disposition: Disposition,
    disposition_at_ms: Option<u64>,
    verifier_class: VerifierClass,
    evidence_disposition: EvidenceDisposition,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Disposition {
    Accepted,
    Rejected,
    Unknown,
    NotInspected,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum VerifierClass {
    Independent,
    Producer,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum EvidenceDisposition {
    Admitted,
    Unknown,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct CaseScore {
    case_id: String,
    scenario_family_id: String,
    produced_units: u64,
    independently_accepted_units: u64,
    independently_rejected_units: u64,
    unresolved_units: u64,
    unqualified_acceptance_claim_units: u64,
    time_to_first_accepted_output_ms: Option<u64>,
    time_to_recovery_target_ms: Option<u64>,
    accepted_demand_fill_ppm: u64,
    accepted_quality_yield_ppm: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct FamilyScore {
    scenario_family_id: String,
    case_count: usize,
    cases_with_first_accepted_output: usize,
    cases_without_first_accepted_output: usize,
    cases_reaching_recovery_target: usize,
    cases_not_reaching_recovery_target: usize,
    produced_units_total: u64,
    independently_accepted_units_total: u64,
    independently_rejected_units_total: u64,
    unresolved_units_total: u64,
    unqualified_acceptance_claim_units_total: u64,
    worst_time_to_first_accepted_output_ms: Option<u64>,
    worst_time_to_recovery_target_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct Report {
    benchmark_id: String,
    evaluator_version: &'static str,
    evidence_class: String,
    case_count: usize,
    family_count: usize,
    cases: Vec<CaseScore>,
    families: Vec<FamilyScore>,
    physical_acceptance_verified: bool,
    production_release_authorized: bool,
    machine_commands_issued: bool,
    resource_reservations_created: bool,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct KnownAnswerFile {
    #[serde(rename = "$schema")]
    json_schema: String,
    schema_id: String,
    benchmark_id: String,
    evidence_class: String,
    answer_count: usize,
    answers: Vec<KnownAnswer>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct KnownAnswer {
    case_id: String,
    produced_units: u64,
    independently_accepted_units: u64,
    independently_rejected_units: u64,
    unresolved_units: u64,
    unqualified_acceptance_claim_units: u64,
    time_to_first_accepted_output_ms: Option<u64>,
    time_to_recovery_target_ms: Option<u64>,
    accepted_demand_fill_ppm: u64,
    accepted_quality_yield_ppm: Option<u64>,
}

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn validate_schema(instance: &serde_json::Value, schema_source: &str) -> Result<(), String> {
    let schema: serde_json::Value = serde_json::from_str(schema_source)
        .map_err(|error| fail(format!("schema_parse:{error}")))?;
    let validator = jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| fail(format!("schema_compile:{error}")))?;
    validator
        .validate(instance)
        .map_err(|error| fail(format!("schema_validation:{error}")))
}

fn independently_qualified(output: &OutputRecord) -> bool {
    output.verifier_class == VerifierClass::Independent
        && output.evidence_disposition == EvidenceDisposition::Admitted
}

fn validate_case(case: &EpisodeCase) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.scenario_family_id.trim().is_empty()
        || case.horizon_end_ms <= case.disturbance_at_ms
        || case.demand_units == 0
        || case.recovery_target_units == 0
        || case.recovery_target_units > case.demand_units
    {
        return Err(fail(format!(
            "case_{}_identity_or_episode_invalid",
            case.case_id
        )));
    }

    let mut output_ids = BTreeSet::new();
    let mut product_unit_ids = BTreeSet::new();
    for output in &case.outputs {
        if output.output_id.trim().is_empty()
            || !output_ids.insert(output.output_id.as_str())
            || output.unit_ids.is_empty()
            || output.produced_at_ms < case.disturbance_at_ms
            || output.produced_at_ms > case.horizon_end_ms
        {
            return Err(fail(format!(
                "case_{}_output_identity_units_or_production_time_invalid",
                case.case_id
            )));
        }
        for unit_id in &output.unit_ids {
            if unit_id.trim().is_empty()
                || unit_id.trim() != unit_id.as_str()
                || !product_unit_ids.insert(unit_id.as_str())
            {
                return Err(fail("duplicate_or_invalid_product_unit_id"));
            }
        }

        let disposition_requires_time = matches!(
            output.disposition,
            Disposition::Accepted | Disposition::Rejected
        );
        if disposition_requires_time && output.disposition_at_ms.is_none() {
            return Err(fail(format!(
                "case_{}_output_{}_disposition_time_required",
                case.case_id, output.output_id
            )));
        }
        if let Some(disposition_at_ms) = output.disposition_at_ms
            && (disposition_at_ms < output.produced_at_ms
                || disposition_at_ms > case.horizon_end_ms)
        {
            return Err(fail(format!(
                "case_{}_output_{}_disposition_time_outside_episode",
                case.case_id, output.output_id
            )));
        }
    }
    Ok(())
}

fn validate_corpus(corpus: &Corpus) -> Result<(), String> {
    if corpus.json_schema != "https://json-schema.org/draft/2020-12/schema"
        || corpus.schema_id != "mfg-accepted-output-scorecard-cases-v0.1"
        || corpus.benchmark_id != "MFG-ACCEPTED-OUTPUT-SCORECARD-001"
        || corpus.evidence_class != "synthetic_offline"
        || corpus.claim_ceiling.trim().is_empty()
        || corpus.case_count != corpus.cases.len()
        || corpus.case_count < 3
    {
        return Err(fail("corpus_metadata_or_case_count_invalid"));
    }

    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err(fail("duplicate_case_id"));
        }
        validate_case(case)?;
    }
    Ok(())
}

fn checked_add(total: u64, value: u64, context: &str) -> Result<u64, String> {
    total
        .checked_add(value)
        .ok_or_else(|| fail(format!("{context}_overflow")))
}

fn ratio_ppm(numerator: u64, denominator: u64) -> u64 {
    ((u128::from(numerator) * PPM_SCALE) / u128::from(denominator)) as u64
}

fn evaluate_case(case: &EpisodeCase) -> Result<CaseScore, String> {
    let mut produced_units = 0_u64;
    let mut independently_accepted_units = 0_u64;
    let mut independently_rejected_units = 0_u64;
    let mut unqualified_acceptance_claim_units = 0_u64;
    let mut accepted_events: Vec<(u64, String, u64)> = Vec::new();

    for output in &case.outputs {
        let output_units = u64::try_from(output.unit_ids.len())
            .map_err(|_| fail("output_unit_count_overflow"))?;
        produced_units = checked_add(produced_units, output_units, "produced_units")?;
        let qualified = independently_qualified(output);
        match (output.disposition, qualified) {
            (Disposition::Accepted, true) => {
                independently_accepted_units =
                    checked_add(independently_accepted_units, output_units, "accepted_units")?;
                let accepted_at = output.disposition_at_ms.ok_or_else(|| {
                    fail("accepted_output_missing_disposition_time_after_validation")
                })?;
                accepted_events.push((accepted_at, output.output_id.clone(), output_units));
            }
            (Disposition::Rejected, true) => {
                independently_rejected_units =
                    checked_add(independently_rejected_units, output_units, "rejected_units")?;
            }
            (Disposition::Accepted, false) => {
                unqualified_acceptance_claim_units = checked_add(
                    unqualified_acceptance_claim_units,
                    output_units,
                    "unqualified_acceptance_claim_units",
                )?;
            }
            _ => {}
        }
    }

    let accounted_units = independently_accepted_units
        .checked_add(independently_rejected_units)
        .ok_or_else(|| fail("accounted_units_overflow"))?;
    let unresolved_units = produced_units
        .checked_sub(accounted_units)
        .ok_or_else(|| fail("qualified_disposition_exceeds_produced_units"))?;

    accepted_events.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let first_accepted_at = accepted_events.first().map(|event| event.0);
    let time_to_first_accepted_output_ms = first_accepted_at.map(|at| at - case.disturbance_at_ms);

    let mut cumulative_accepted = 0_u64;
    let mut recovery_at = None;
    for (accepted_at, _, units) in &accepted_events {
        cumulative_accepted =
            checked_add(cumulative_accepted, *units, "cumulative_accepted_units")?;
        if cumulative_accepted >= case.recovery_target_units {
            recovery_at = Some(*accepted_at);
            break;
        }
    }
    let time_to_recovery_target_ms = recovery_at.map(|at| at - case.disturbance_at_ms);

    let accepted_demand_fill_ppm = ratio_ppm(
        independently_accepted_units.min(case.demand_units),
        case.demand_units,
    );
    let accepted_quality_yield_ppm =
        (produced_units > 0).then(|| ratio_ppm(independently_accepted_units, produced_units));

    Ok(CaseScore {
        case_id: case.case_id.clone(),
        scenario_family_id: case.scenario_family_id.clone(),
        produced_units,
        independently_accepted_units,
        independently_rejected_units,
        unresolved_units,
        unqualified_acceptance_claim_units,
        time_to_first_accepted_output_ms,
        time_to_recovery_target_ms,
        accepted_demand_fill_ppm,
        accepted_quality_yield_ppm,
    })
}

fn checked_sum<F>(scores: &[CaseScore], get: F, context: &str) -> Result<u64, String>
where
    F: Fn(&CaseScore) -> u64,
{
    scores
        .iter()
        .try_fold(0_u64, |sum, score| checked_add(sum, get(score), context))
}

fn evaluate(corpus: &Corpus) -> Result<Report, String> {
    let mut cases = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        cases.push(evaluate_case(case)?);
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let mut family_groups: BTreeMap<String, Vec<CaseScore>> = BTreeMap::new();
    for score in &cases {
        family_groups
            .entry(score.scenario_family_id.clone())
            .or_default()
            .push(score.clone());
    }

    let mut families = Vec::with_capacity(family_groups.len());
    for (scenario_family_id, group) in family_groups {
        let cases_with_first = group
            .iter()
            .filter(|score| score.time_to_first_accepted_output_ms.is_some())
            .count();
        let cases_reaching_target = group
            .iter()
            .filter(|score| score.time_to_recovery_target_ms.is_some())
            .count();

        families.push(FamilyScore {
            scenario_family_id,
            case_count: group.len(),
            cases_with_first_accepted_output: cases_with_first,
            cases_without_first_accepted_output: group.len() - cases_with_first,
            cases_reaching_recovery_target: cases_reaching_target,
            cases_not_reaching_recovery_target: group.len() - cases_reaching_target,
            produced_units_total: checked_sum(
                &group,
                |score| score.produced_units,
                "family_produced",
            )?,
            independently_accepted_units_total: checked_sum(
                &group,
                |score| score.independently_accepted_units,
                "family_accepted",
            )?,
            independently_rejected_units_total: checked_sum(
                &group,
                |score| score.independently_rejected_units,
                "family_rejected",
            )?,
            unresolved_units_total: checked_sum(
                &group,
                |score| score.unresolved_units,
                "family_unresolved",
            )?,
            unqualified_acceptance_claim_units_total: checked_sum(
                &group,
                |score| score.unqualified_acceptance_claim_units,
                "family_unqualified",
            )?,
            worst_time_to_first_accepted_output_ms: group
                .iter()
                .filter_map(|score| score.time_to_first_accepted_output_ms)
                .max(),
            worst_time_to_recovery_target_ms: group
                .iter()
                .filter_map(|score| score.time_to_recovery_target_ms)
                .max(),
        });
    }

    Ok(Report {
        benchmark_id: corpus.benchmark_id.clone(),
        evaluator_version: "0.1.0",
        evidence_class: corpus.evidence_class.clone(),
        case_count: cases.len(),
        family_count: families.len(),
        cases,
        families,
        physical_acceptance_verified: false,
        production_release_authorized: false,
        machine_commands_issued: false,
        resource_reservations_created: false,
        claim_ceiling: corpus.claim_ceiling.clone(),
    })
}

fn to_known_answer(score: &CaseScore) -> KnownAnswer {
    KnownAnswer {
        case_id: score.case_id.clone(),
        produced_units: score.produced_units,
        independently_accepted_units: score.independently_accepted_units,
        independently_rejected_units: score.independently_rejected_units,
        unresolved_units: score.unresolved_units,
        unqualified_acceptance_claim_units: score.unqualified_acceptance_claim_units,
        time_to_first_accepted_output_ms: score.time_to_first_accepted_output_ms,
        time_to_recovery_target_ms: score.time_to_recovery_target_ms,
        accepted_demand_fill_ppm: score.accepted_demand_fill_ppm,
        accepted_quality_yield_ppm: score.accepted_quality_yield_ppm,
    }
}

fn verify_known_answers(corpus: &Corpus, report: &Report) -> Result<(), String> {
    let answers_value: serde_json::Value = serde_json::from_str(ANSWERS_JSON)
        .map_err(|error| fail(format!("known_answers_parse:{error}")))?;
    validate_schema(&answers_value, ANSWERS_SCHEMA_JSON)?;
    let answer_file: KnownAnswerFile = serde_json::from_value(answers_value)
        .map_err(|error| fail(format!("known_answers_deserialize:{error}")))?;

    if answer_file.json_schema != "https://json-schema.org/draft/2020-12/schema"
        || answer_file.schema_id != "mfg-accepted-output-scorecard-known-answers-v0.1"
        || answer_file.benchmark_id != corpus.benchmark_id
        || answer_file.evidence_class != corpus.evidence_class
        || answer_file.claim_ceiling.trim().is_empty()
        || answer_file.answer_count != answer_file.answers.len()
        || answer_file.answer_count != report.cases.len()
    {
        return Err(fail("known_answer_metadata_or_count_invalid"));
    }

    let mut expected = answer_file.answers;
    expected.sort();
    let mut actual: Vec<KnownAnswer> = report.cases.iter().map(to_known_answer).collect();
    actual.sort();
    if actual != expected {
        return Err(fail("known_answer_mismatch"));
    }
    Ok(())
}

fn build_report() -> Result<Report, String> {
    let corpus_value: serde_json::Value =
        serde_json::from_str(CASES_JSON).map_err(|error| fail(format!("cases_parse:{error}")))?;
    validate_schema(&corpus_value, CASES_SCHEMA_JSON)?;
    let corpus: Corpus = serde_json::from_value(corpus_value)
        .map_err(|error| fail(format!("cases_deserialize:{error}")))?;
    validate_corpus(&corpus)?;
    let report = evaluate(&corpus)?;
    verify_known_answers(&corpus, &report)?;
    Ok(report)
}

fn main() -> ExitCode {
    match build_report().and_then(|report| {
        serde_json::to_string_pretty(&report)
            .map_err(|error| fail(format!("report_serialize:{error}")))
    }) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("mfg_accepted_output_scorecard_failed:{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_corpus() -> Corpus {
        serde_json::from_str(CASES_JSON).expect("frozen corpus must deserialize")
    }

    #[test]
    fn schemas_and_frozen_known_answers_pass() {
        let report = build_report().expect("frozen scorecard must pass");
        assert_eq!(report.case_count, 4);
        assert_eq!(report.family_count, 4);
        assert!(!report.physical_acceptance_verified);
        assert!(!report.production_release_authorized);
        assert!(!report.machine_commands_issued);
        assert!(!report.resource_reservations_created);
    }

    #[test]
    fn only_independently_verified_admitted_acceptance_counts() {
        let report = build_report().expect("frozen scorecard must pass");
        let nominal = report
            .cases
            .iter()
            .find(|score| score.case_id == "CASE-NOMINAL")
            .expect("nominal case exists");
        assert_eq!(nominal.produced_units, 17);
        assert_eq!(nominal.independently_accepted_units, 7);
        assert_eq!(nominal.independently_rejected_units, 2);
        assert_eq!(nominal.unresolved_units, 8);
        assert_eq!(nominal.unqualified_acceptance_claim_units, 6);
    }

    #[test]
    fn zero_accepted_output_is_not_reported_as_zero_time_recovery() {
        let report = build_report().expect("frozen scorecard must pass");
        let stalled = report
            .cases
            .iter()
            .find(|score| score.case_id == "CASE-NO-ACCEPTANCE")
            .expect("stalled case exists");
        assert_eq!(stalled.independently_accepted_units, 0);
        assert_eq!(stalled.time_to_first_accepted_output_ms, None);
        assert_eq!(stalled.time_to_recovery_target_ms, None);
        assert_eq!(stalled.accepted_demand_fill_ppm, 0);
    }

    #[test]
    fn zero_production_has_no_defined_quality_yield() {
        let report = build_report().expect("frozen scorecard must pass");
        let empty = report
            .cases
            .iter()
            .find(|score| score.case_id == "CASE-NO-OUTPUT")
            .expect("empty case exists");
        assert_eq!(empty.produced_units, 0);
        assert_eq!(empty.accepted_quality_yield_ppm, None);
        assert_eq!(empty.unresolved_units, 0);
    }

    #[test]
    fn case_and_output_order_do_not_change_scorecard() {
        let mut corpus = sample_corpus();
        let expected = evaluate(&corpus).expect("original corpus evaluates");
        corpus.cases.reverse();
        for case in &mut corpus.cases {
            case.outputs.reverse();
        }
        let actual = evaluate(&corpus).expect("permuted corpus evaluates");
        assert_eq!(actual, expected);
    }

    #[test]
    fn duplicate_case_ids_fail_closed() {
        let mut corpus = sample_corpus();
        let duplicate_id = corpus.cases[0].case_id.clone();
        corpus.cases[1].case_id = duplicate_id;
        assert_eq!(
            validate_corpus(&corpus),
            Err("duplicate_case_id".to_owned())
        );
    }

    #[test]
    fn duplicate_output_ids_fail_closed() {
        let mut corpus = sample_corpus();
        let duplicate_id = corpus.cases[0].outputs[0].output_id.clone();
        corpus.cases[0].outputs[1].output_id = duplicate_id;
        assert!(validate_case(&corpus.cases[0]).is_err());
    }

    #[test]
    fn duplicate_product_unit_ids_across_outputs_fail_closed() {
        let mut corpus = sample_corpus();
        let duplicate_id = corpus.cases[0].outputs[0].unit_ids[0].clone();
        corpus.cases[0].outputs[1].unit_ids[0] = duplicate_id;
        assert_eq!(
            validate_case(&corpus.cases[0]),
            Err("duplicate_or_invalid_product_unit_id".to_owned())
        );
    }

    #[test]
    fn empty_product_unit_ids_fail_closed() {
        let mut corpus = sample_corpus();
        corpus.cases[0].outputs[0].unit_ids.clear();
        assert!(validate_case(&corpus.cases[0]).is_err());
    }

    #[test]
    fn disposition_before_production_fails_closed() {
        let mut corpus = sample_corpus();
        let invalid_disposition_at_ms = corpus.cases[0].outputs[0].produced_at_ms - 1;
        corpus.cases[0].outputs[0].disposition_at_ms = Some(invalid_disposition_at_ms);
        assert!(validate_case(&corpus.cases[0]).is_err());
    }

    #[test]
    fn accepted_or_rejected_output_requires_a_disposition_time() {
        let mut corpus = sample_corpus();
        corpus.cases[0].outputs[0].disposition_at_ms = None;
        assert!(validate_case(&corpus.cases[0]).is_err());
    }

    #[test]
    fn outputs_outside_the_disruption_horizon_fail_closed() {
        let mut corpus = sample_corpus();
        let invalid_produced_at_ms = corpus.cases[0].disturbance_at_ms - 1;
        corpus.cases[0].outputs[0].produced_at_ms = invalid_produced_at_ms;
        assert!(validate_case(&corpus.cases[0]).is_err());
    }

    #[test]
    fn family_score_preserves_unrecovered_episode_counts() {
        let report = build_report().expect("frozen scorecard must pass");
        let quality_family = report
            .families
            .iter()
            .find(|family| family.scenario_family_id == "measurement_unknown")
            .expect("measurement family exists");
        assert_eq!(quality_family.cases_without_first_accepted_output, 1);
        assert_eq!(quality_family.cases_not_reaching_recovery_target, 1);
        assert_eq!(quality_family.worst_time_to_recovery_target_ms, None);
    }
}
