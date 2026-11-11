use jsonschema::{Validator, draft202012};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_VERSION: &str = "0.1.0";
const EPSILON: f64 = 1.0e-9;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema_version: String,
    pub option_set_id: String,
    pub claim_ceiling: String,
    pub service_contract: ServiceContract,
    pub mission_reports: Vec<MissionReport>,
    pub candidates: Vec<Candidate>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceContract {
    pub contract_id: String,
    pub contract_sha256: String,
    pub mission_functional_unit: String,
    pub quantity_unit: String,
    pub minimum_service_quantity: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MissionReport {
    pub report_ref: String,
    pub report_sha256: String,
    pub report: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricBound {
    pub lower: f64,
    pub upper: f64,
    pub unit: String,
    pub boundary: String,
    pub bound_basis: String,
    pub evidence_class: String,
    pub evidence_ref: String,
    pub evidence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDelivery {
    pub lower: f64,
    pub upper: f64,
    pub unit: String,
    pub bound_basis: String,
    pub evidence_class: String,
    pub evidence_ref: String,
    pub evidence_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub candidate_id: String,
    pub label: String,
    pub strategy_class: String,
    pub source_report_ref: String,
    pub source_report_sha256: String,
    pub scenario_id: String,
    pub source_result_sha256: String,
    pub service_delivery: ServiceDelivery,
    pub battery_energy: MetricBound,
    pub elapsed_time: MetricBound,
}

#[derive(Clone, Debug, Serialize)]
pub struct CandidateAudit {
    pub candidate_id: String,
    pub label: String,
    pub strategy_class: String,
    pub source_report_ref: String,
    pub scenario_id: String,
    pub design_id: String,
    pub source_evidence_class: String,
    pub source_mission_status: String,
    pub disposition: String,
    pub frontier_eligible: bool,
    pub service_delivery_lower: f64,
    pub service_delivery_upper: f64,
    pub minimum_service_required: f64,
    pub battery_energy_lower_kwh: f64,
    pub battery_energy_upper_kwh: f64,
    pub battery_energy_nominal_kwh: f64,
    pub elapsed_time_lower_hours: f64,
    pub elapsed_time_upper_hours: f64,
    pub elapsed_time_nominal_hours: f64,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DominancePair {
    pub dominator_candidate_id: String,
    pub dominated_candidate_id: String,
    pub battery_energy_separation_kwh: f64,
    pub elapsed_time_separation_hours: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct AuditReport {
    pub schema_version: String,
    pub option_set_id: String,
    pub canonical_service_contract_sha256: String,
    pub canonical_corpus_sha256: String,
    pub report_sha256: String,
    pub candidate_count: usize,
    pub eligible_candidate_count: usize,
    pub blocked_count: usize,
    pub insufficient_service_count: usize,
    pub indeterminate_count: usize,
    pub robust_dominance_pair_count: usize,
    pub robust_frontier_count: usize,
    pub source_grade: String,
    pub robust_frontier_candidate_ids: Vec<String>,
    pub dominance_pairs: Vec<DominancePair>,
    pub candidates: Vec<CandidateAudit>,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug)]
struct ResolvedCandidate {
    audit: CandidateAudit,
    energy_lower: f64,
    energy_upper: f64,
    elapsed_lower: f64,
    elapsed_upper: f64,
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn compile_schema(bytes: &[u8]) -> Result<Validator, String> {
    let schema: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("schema JSON: {error}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if !draft202012::meta::is_valid(&schema) {
        return Err("schema fails Draft 2020-12 meta-schema validation".to_owned());
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("schema compilation: {error}"))
}

fn validate_schema(validator: &Validator, value: &Value) -> Result<(), String> {
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| error.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "mission option schema errors: {}",
            errors.join("; ")
        ))
    }
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("canonical JSON serialization: {error}"))
}

fn canonical_service_contract(contract: &ServiceContract) -> Result<Vec<u8>, String> {
    let mut value = serde_json::to_value(contract)
        .map_err(|error| format!("service contract serialization: {error}"))?;
    value["contract_sha256"] = Value::String(String::new());
    canonical_json(&value)
}

fn report_result<'a>(report: &'a Value, scenario_id: &str) -> Result<&'a Value, String> {
    let results = report
        .get("results")
        .and_then(Value::as_array)
        .ok_or_else(|| "source mission report has no results array".to_owned())?;
    let found: Vec<&Value> = results
        .iter()
        .filter(|result| result.get("scenario_id").and_then(Value::as_str) == Some(scenario_id))
        .collect();
    if found.len() != 1 {
        return Err(format!(
            "expected exactly one scenario result for {scenario_id}, found {}",
            found.len()
        ));
    }
    Ok(found[0])
}

fn number(value: &Value, field: &str) -> Result<f64, String> {
    let result = value
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("mission result missing numeric field {field}"))?;
    if !result.is_finite() {
        return Err(format!("mission result field {field} is not finite"));
    }
    Ok(result)
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("mission result missing nonempty field {field}"))
}

fn validate_bound(bound: &MetricBound, expected_unit: &str, expected_boundary: &str) -> Result<(), String> {
    if !finite_nonnegative(bound.lower)
        || !finite_nonnegative(bound.upper)
        || bound.lower > bound.upper
        || bound.unit != expected_unit
        || bound.boundary != expected_boundary
        || !["DECLARED_HARD_BOUND", "SCENARIO_ENVELOPE"].contains(&bound.bound_basis.as_str())
        || !["synthetic", "measured", "manufacturer", "literature", "estimate", "unknown"]
            .contains(&bound.evidence_class.as_str())
        || !nonempty(&bound.evidence_ref)
        || !is_sha256(&bound.evidence_sha256)
    {
        return Err(format!(
            "invalid or incomparable metric bound for {expected_boundary}"
        ));
    }
    Ok(())
}

fn validate_service_delivery(service: &ServiceDelivery, contract: &ServiceContract) -> Result<(), String> {
    if !finite_nonnegative(service.lower)
        || !finite_nonnegative(service.upper)
        || service.lower > service.upper
        || service.unit != contract.quantity_unit
        || !["DECLARED_HARD_BOUND", "SCENARIO_ENVELOPE"].contains(&service.bound_basis.as_str())
        || !["synthetic", "measured", "manufacturer", "literature", "estimate", "unknown"]
            .contains(&service.evidence_class.as_str())
        || !nonempty(&service.evidence_ref)
        || !is_sha256(&service.evidence_sha256)
    {
        return Err("invalid or incomparable service-delivery bounds".to_owned());
    }
    Ok(())
}

fn resolve_candidate(
    candidate: &Candidate,
    contract: &ServiceContract,
    reports: &BTreeMap<&str, &MissionReport>,
) -> Result<ResolvedCandidate, String> {
    if !nonempty(&candidate.candidate_id)
        || !nonempty(&candidate.label)
        || ![
            "baseline",
            "route_profile",
            "speed_profile",
            "auxiliary_management",
            "load_shed",
        ]
        .contains(&candidate.strategy_class.as_str())
    {
        return Err(format!("candidate {} has invalid identity/strategy class", candidate.candidate_id));
    }
    let source = reports
        .get(candidate.source_report_ref.as_str())
        .ok_or_else(|| format!("candidate {} references unknown mission report", candidate.candidate_id))?;
    if candidate.source_report_sha256 != source.report_sha256 {
        return Err(format!("candidate {} source mission report digest mismatch", candidate.candidate_id));
    }
    let actual_report_digest = sha256(&canonical_json(&source.report)?);
    if actual_report_digest != source.report_sha256 {
        return Err(format!("source mission report {} does not match its declared digest", source.report_ref));
    }
    if string(&source.report, "schema_version")? != "0.2.0"
        || string(&source.report, "catalog_id")? != "EV-PLATFORM-001"
        || string(&source.report, "profile_set_id")? != "EV-MISSION-PHYSICS-001"
    {
        return Err(format!("source report {} is not the supported vehicle mission physics report", source.report_ref));
    }
    let result = report_result(&source.report, &candidate.scenario_id)?;
    if sha256(&canonical_json(result)?) != candidate.source_result_sha256 {
        return Err(format!("candidate {} source scenario result digest mismatch", candidate.candidate_id));
    }
    let scenario_id = string(result, "scenario_id")?;
    let design_id = string(result, "design_id")?;
    let source_evidence = string(result, "evidence_class")?;
    let mission_unit = string(result, "mission_functional_unit")?;
    let mission_status = string(result, "status")?;
    if mission_unit != contract.mission_functional_unit {
        return Err(format!(
            "candidate {} mission functional unit does not match the service contract",
            candidate.candidate_id
        ));
    }
    let gross_margin = number(result, "gross_mass_margin_kg")?;
    let power_margin = number(result, "peak_power_margin_kw")?;
    let energy_margin = number(result, "energy_margin_vs_usable_capacity_kwh")?;
    let unserved = number(result, "total_unserved_energy_kwh")?;
    let energy_nominal = number(result, "net_battery_energy_demand_kwh")?;
    let travel = number(result, "total_travel_hours")?;
    let stationary = number(result, "total_stationary_segment_hours")?;
    let nontravel = number(result, "nontravel_duration_hours")?;
    let elapsed_nominal = travel + stationary + nontravel;
    if ![
        "synthetic", "measured", "manufacturer", "literature", "estimate", "unknown",
    ]
    .contains(&source_evidence)
    {
        return Err(format!("candidate {} has an unknown mission evidence class", candidate.candidate_id));
    }
    if energy_nominal < -EPSILON
        || ![travel, stationary, nontravel].iter().all(|value| finite_nonnegative(*value))
        || !elapsed_nominal.is_finite()
        || elapsed_nominal <= 0.0
    {
        return Err(format!("candidate {} has invalid energy or duration in the source mission result", candidate.candidate_id));
    }

    validate_bound(
        &candidate.battery_energy,
        "kWh",
        "BATTERY_TERMINAL_NET",
    )?;
    validate_bound(
        &candidate.elapsed_time,
        "h",
        "VEHICLE_OUT_THE_DOOR_ELAPSED",
    )?;
    validate_service_delivery(&candidate.service_delivery, contract)?;

    if candidate.battery_energy.lower > energy_nominal + EPSILON
        || candidate.battery_energy.upper < energy_nominal - EPSILON
    {
        return Err(format!(
            "candidate {} battery-energy uncertainty bounds do not contain the mission-screen nominal value",
            candidate.candidate_id
        ));
    }
    if candidate.elapsed_time.lower > elapsed_nominal + EPSILON
        || candidate.elapsed_time.upper < elapsed_nominal - EPSILON
    {
        return Err(format!(
            "candidate {} elapsed-time uncertainty bounds do not contain the mission-screen nominal value",
            candidate.candidate_id
        ));
    }

    let mut reasons = Vec::new();
    let service_passes = candidate.service_delivery.lower + EPSILON >= contract.minimum_service_quantity;
    if !service_passes {
        reasons.push("conservative_service_delivery_below_contract_minimum".to_owned());
    }
    let energy_physics_pass = gross_margin >= -EPSILON
        && power_margin >= -EPSILON
        && energy_margin >= -EPSILON
        && unserved <= EPSILON;
    if mission_status != "blocked" && mission_status != "indeterminate" && !energy_physics_pass {
        return Err(format!(
            "candidate {} mission report status contradicts mass/power/energy hard-gate fields",
            candidate.candidate_id
        ));
    }
    let bound_unknown = candidate.service_delivery.evidence_class == "unknown"
        || candidate.battery_energy.evidence_class == "unknown"
        || candidate.elapsed_time.evidence_class == "unknown";
    let disposition = if !service_passes {
        "INSUFFICIENT_SERVICE"
    } else if mission_status == "blocked" {
        reasons.push("mission_physics_screen_blocked".to_owned());
        "BLOCKED_BY_MISSION_SCREEN"
    } else if mission_status == "indeterminate" || bound_unknown || source_evidence == "unknown" {
        reasons.push("mission_or_metric_evidence_indeterminate".to_owned());
        "INDETERMINATE"
    } else if mission_status == "passes_synthetic_screen" {
        if source_evidence != "synthetic" {
            return Err(format!(
                "candidate {} has passes_synthetic_screen with a non-synthetic evidence class",
                candidate.candidate_id
            ));
        }
        "ELIGIBLE_SYNTHETIC_SCREEN"
    } else if mission_status == "passes_evidence_backed_screen" {
        if source_evidence == "synthetic" || source_evidence == "unknown" {
            return Err(format!(
                "candidate {} has evidence-backed status with synthetic/unknown source evidence",
                candidate.candidate_id
            ));
        }
        "ELIGIBLE_EVIDENCE_BACKED_SCREEN_NOT_VALIDATED"
    } else {
        return Err(format!(
            "candidate {} has unsupported mission-screen status {mission_status}",
            candidate.candidate_id
        ));
    }
    .to_owned();

    Ok(ResolvedCandidate {
        audit: CandidateAudit {
            candidate_id: candidate.candidate_id.clone(),
            label: candidate.label.clone(),
            strategy_class: candidate.strategy_class.clone(),
            source_report_ref: candidate.source_report_ref.clone(),
            scenario_id: scenario_id.to_owned(),
            design_id: design_id.to_owned(),
            source_evidence_class: source_evidence.to_owned(),
            source_mission_status: mission_status.to_owned(),
            disposition,
            frontier_eligible: matches!(
                disposition.as_str(),
                "ELIGIBLE_SYNTHETIC_SCREEN" | "ELIGIBLE_EVIDENCE_BACKED_SCREEN_NOT_VALIDATED"
            ),
            service_delivery_lower: candidate.service_delivery.lower,
            service_delivery_upper: candidate.service_delivery.upper,
            minimum_service_required: contract.minimum_service_quantity,
            battery_energy_lower_kwh: candidate.battery_energy.lower,
            battery_energy_upper_kwh: candidate.battery_energy.upper,
            battery_energy_nominal_kwh: energy_nominal,
            elapsed_time_lower_hours: candidate.elapsed_time.lower,
            elapsed_time_upper_hours: candidate.elapsed_time.upper,
            elapsed_time_nominal_hours: elapsed_nominal,
            reasons,
        },
        energy_lower: candidate.battery_energy.lower,
        energy_upper: candidate.battery_energy.upper,
        elapsed_lower: candidate.elapsed_time.lower,
        elapsed_upper: candidate.elapsed_time.upper,
    })
}

fn robustly_dominates(left: &ResolvedCandidate, right: &ResolvedCandidate) -> bool {
    let energy_no_worse = left.energy_upper <= right.energy_lower + EPSILON;
    let elapsed_no_worse = left.elapsed_upper <= right.elapsed_lower + EPSILON;
    let strictly_better = left.energy_upper < right.energy_lower - EPSILON
        || left.elapsed_upper < right.elapsed_lower - EPSILON;
    energy_no_worse && elapsed_no_worse && strictly_better
}

fn canonicalize(corpus: &mut Corpus) {
    corpus
        .mission_reports
        .sort_by(|left, right| left.report_ref.cmp(&right.report_ref));
    corpus
        .candidates
        .sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
}

pub fn audit_corpus(schema_bytes: &[u8], corpus_bytes: &[u8]) -> Result<AuditReport, String> {
    let validator = compile_schema(schema_bytes)?;
    let value: Value =
        serde_json::from_slice(corpus_bytes).map_err(|error| format!("corpus JSON: {error}"))?;
    validate_schema(&validator, &value)?;
    let mut corpus: Corpus =
        serde_json::from_value(value).map_err(|error| format!("corpus decode: {error}"))?;
    if corpus.schema_version != CORPUS_VERSION
        || !nonempty(&corpus.option_set_id)
        || !nonempty(&corpus.claim_ceiling)
        || corpus.candidates.len() < 2
    {
        return Err("unsupported, incomplete or undersized mission option set".to_owned());
    }
    if !nonempty(&corpus.service_contract.contract_id)
        || !nonempty(&corpus.service_contract.mission_functional_unit)
        || !nonempty(&corpus.service_contract.quantity_unit)
        || !finite_nonnegative(corpus.service_contract.minimum_service_quantity)
        || corpus.service_contract.minimum_service_quantity <= 0.0
        || !is_sha256(&corpus.service_contract.contract_sha256)
    {
        return Err("service contract identity, unit, minimum or digest is invalid".to_owned());
    }
    let actual_contract_digest = sha256(&canonical_service_contract(&corpus.service_contract)?);
    if actual_contract_digest != corpus.service_contract.contract_sha256 {
        return Err("service contract digest does not match its canonical contents".to_owned());
    }

    let mut reports = BTreeMap::new();
    for report in &corpus.mission_reports {
        if !nonempty(&report.report_ref)
            || !is_sha256(&report.report_sha256)
            || reports.insert(report.report_ref.as_str(), report).is_some()
        {
            return Err("mission report references must be unique and carry SHA-256 digests".to_owned());
        }
    }
    let mut candidate_ids = BTreeSet::new();
    let mut source_results = BTreeSet::new();
    for candidate in &corpus.candidates {
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(format!("duplicate candidate identity: {}", candidate.candidate_id));
        }
        if !source_results.insert((
            candidate.source_report_ref.clone(),
            candidate.scenario_id.clone(),
        )) {
            return Err(format!(
                "mission scenario result is reused by multiple alternatives: {} / {}",
                candidate.source_report_ref, candidate.scenario_id
            ));
        }
    }

    let mut resolved = Vec::with_capacity(corpus.candidates.len());
    for candidate in &corpus.candidates {
        resolved.push(resolve_candidate(
            candidate,
            &corpus.service_contract,
            &reports,
        )?);
    }
    resolved.sort_by(|left, right| left.audit.candidate_id.cmp(&right.audit.candidate_id));

    let eligible: Vec<&ResolvedCandidate> = resolved
        .iter()
        .filter(|candidate| candidate.audit.frontier_eligible)
        .collect();
    let mut dominance_pairs = Vec::new();
    let mut dominated = BTreeSet::new();
    for left in &eligible {
        for right in &eligible {
            if left.audit.candidate_id == right.audit.candidate_id {
                continue;
            }
            if robustly_dominates(left, right) {
                dominated.insert(right.audit.candidate_id.clone());
                dominance_pairs.push(DominancePair {
                    dominator_candidate_id: left.audit.candidate_id.clone(),
                    dominated_candidate_id: right.audit.candidate_id.clone(),
                    battery_energy_separation_kwh: right.energy_lower - left.energy_upper,
                    elapsed_time_separation_hours: right.elapsed_lower - left.elapsed_upper,
                });
            }
        }
    }
    dominance_pairs.sort_by(|left, right| {
        (&left.dominator_candidate_id, &left.dominated_candidate_id)
            .cmp(&(&right.dominator_candidate_id, &right.dominated_candidate_id))
    });
    let mut frontier_ids: Vec<String> = eligible
        .iter()
        .filter(|candidate| !dominated.contains(&candidate.audit.candidate_id))
        .map(|candidate| candidate.audit.candidate_id.clone())
        .collect();
    frontier_ids.sort();

    let count = |disposition: &str| {
        resolved
            .iter()
            .filter(|candidate| candidate.audit.disposition == disposition)
            .count()
    };
    let source_grade = if resolved
        .iter()
        .all(|candidate| candidate.audit.source_evidence_class == "synthetic")
    {
        "SYNTHETIC_ONLY"
    } else if resolved.iter().any(|candidate| {
        candidate.audit.source_evidence_class == "unknown"
            || candidate.audit.disposition == "INDETERMINATE"
    }) {
        "MIXED_OR_INDETERMINATE"
    } else {
        "EVIDENCE_BACKED_SCREEN_NOT_PHYSICAL_VALIDATION"
    };

    let mut canonical = corpus.clone();
    canonicalize(&mut canonical);
    let canonical_bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("canonical corpus serialization: {error}"))?;
    let mut report = AuditReport {
        schema_version: CORPUS_VERSION.to_owned(),
        option_set_id: corpus.option_set_id.clone(),
        canonical_service_contract_sha256: actual_contract_digest,
        canonical_corpus_sha256: sha256(&canonical_bytes),
        report_sha256: String::new(),
        candidate_count: resolved.len(),
        eligible_candidate_count: eligible.len(),
        blocked_count: count("BLOCKED_BY_MISSION_SCREEN"),
        insufficient_service_count: count("INSUFFICIENT_SERVICE"),
        indeterminate_count: count("INDETERMINATE"),
        robust_dominance_pair_count: dominance_pairs.len(),
        robust_frontier_count: frontier_ids.len(),
        source_grade: source_grade.to_owned(),
        robust_frontier_candidate_ids: frontier_ids,
        dominance_pairs,
        candidates: resolved.into_iter().map(|candidate| candidate.audit).collect(),
        claim_ceiling: corpus.claim_ceiling,
    };
    let report_bytes = serde_json::to_vec(&report)
        .map_err(|error| format!("report serialization: {error}"))?;
    report.report_sha256 = sha256(&report_bytes);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &[u8] = include_bytes!("../../../tools/ev-mission-option-frontier/schemas/mission_option_frontier_v0_1.schema.json");
    const CORPUS: &[u8] = include_bytes!("../../../tools/ev-mission-option-frontier/fixtures/mission_option_frontier_v0_1.json");

    fn report() -> AuditReport {
        audit_corpus(SCHEMA, CORPUS).expect("frozen mission option corpus")
    }

    fn candidate<'a>(report: &'a AuditReport, id: &str) -> &'a CandidateAudit {
        report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == id)
            .expect("candidate audit")
    }

    #[test]
    fn frozen_fixture_keeps_a_full_service_preserving_frontier() {
        let report = report();
        assert_eq!(report.candidate_count, 8);
        assert_eq!(report.eligible_candidate_count, 5);
        assert_eq!(report.blocked_count, 1);
        assert_eq!(report.insufficient_service_count, 1);
        assert_eq!(report.indeterminate_count, 1);
        assert_eq!(report.robust_dominance_pair_count, 2);
        assert_eq!(
            report.robust_frontier_candidate_ids,
            vec![
                "baseline".to_owned(),
                "eco-route".to_owned(),
                "fast-route".to_owned()
            ]
        );
        assert_eq!(report.source_grade, "SYNTHETIC_ONLY");
    }

    #[test]
    fn a_lower_energy_load_shed_option_is_excluded_if_service_is_short() {
        let report = report();
        let candidate = candidate(&report, "load-shed");
        assert_eq!(candidate.disposition, "INSUFFICIENT_SERVICE");
        assert!(!candidate.frontier_eligible);
        assert!(candidate.reasons.iter().any(|reason| reason.contains("service")));
    }

    #[test]
    fn screen_blocked_and_indeterminate_options_never_enter_the_frontier() {
        let report = report();
        assert_eq!(candidate(&report, "blocked").disposition, "BLOCKED_BY_MISSION_SCREEN");
        assert_eq!(candidate(&report, "unknown").disposition, "INDETERMINATE");
        assert!(!candidate(&report, "blocked").frontier_eligible);
        assert!(!candidate(&report, "unknown").frontier_eligible);
    }

    #[test]
    fn robust_frontier_has_no_single_weighted_scalar_winner() {
        let report = report();
        assert!(report.robust_frontier_candidate_ids.contains(&"baseline".to_owned()));
        assert!(report.robust_frontier_candidate_ids.contains(&"eco-route".to_owned()));
        assert!(report.robust_frontier_candidate_ids.contains(&"fast-route".to_owned()));
    }

    #[test]
    fn robust_dominance_requires_nonoverlapping_bounds_on_every_objective() {
        let report = report();
        assert!(report.dominance_pairs.iter().any(|pair| {
            pair.dominator_candidate_id == "baseline" && pair.dominated_candidate_id == "wasteful"
        }));
        assert!(!report.dominance_pairs.iter().any(|pair| {
            pair.dominator_candidate_id == "baseline" && pair.dominated_candidate_id == "eco-route"
        }));
    }

    #[test]
    fn overlapping_bounds_do_not_create_a_nominal_winner() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        let candidates = corpus["candidates"].as_array_mut().unwrap();
        let baseline = candidates.iter_mut().find(|candidate| candidate["candidate_id"] == "baseline").unwrap();
        baseline["battery_energy"]["lower"] = json!(19.5);
        baseline["battery_energy"]["upper"] = json!(20.5);
        baseline["elapsed_time"]["lower"] = json!(1.0);
        baseline["elapsed_time"]["upper"] = json!(1.5);
        baseline["battery_energy"]["evidence_sha256"] = Value::String("f".repeat(64));
        baseline["elapsed_time"]["evidence_sha256"] = Value::String("f".repeat(64));
        let bytes = serde_json::to_vec(&corpus).expect("overlapping corpus");
        let audit = audit_corpus(SCHEMA, &bytes).expect("overlapping interval audit");
        assert!(audit.robust_frontier_candidate_ids.contains(&"baseline".to_owned()));
        assert!(!audit.dominance_pairs.iter().any(|pair| pair.dominated_candidate_id == "baseline"));
    }

    #[test]
    fn load_shed_service_bounds_must_not_be_promoted_by_energy_savings() {
        let audit = report();
        let baseline = candidate(&audit, "baseline");
        let shed = candidate(&audit, "load-shed");
        assert!(shed.battery_energy_nominal_kwh < baseline.battery_energy_nominal_kwh);
        assert!(shed.service_delivery_lower < shed.minimum_service_required);
        assert!(!shed.frontier_eligible);
    }

    #[test]
    fn candidate_order_does_not_change_canonical_report() {
        let original = report();
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        corpus["candidates"].as_array_mut().unwrap().reverse();
        corpus["mission_reports"].as_array_mut().unwrap().reverse();
        let bytes = serde_json::to_vec(&corpus).expect("permuted corpus");
        let replay = audit_corpus(SCHEMA, &bytes).expect("permuted option audit");
        assert_eq!(original.canonical_corpus_sha256, replay.canonical_corpus_sha256);
        assert_eq!(original.report_sha256, replay.report_sha256);
    }

    #[test]
    fn a_changed_source_mission_report_digest_is_rejected() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        corpus["candidates"][0]["source_report_sha256"] = Value::String("f".repeat(64));
        let bytes = serde_json::to_vec(&corpus).expect("tampered report reference");
        assert!(audit_corpus(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn a_changed_source_scenario_result_digest_is_rejected() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        corpus["candidates"][0]["source_result_sha256"] = Value::String("f".repeat(64));
        let bytes = serde_json::to_vec(&corpus).expect("tampered result reference");
        assert!(audit_corpus(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn battery_energy_boundary_must_be_battery_terminal_net() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        corpus["candidates"][0]["battery_energy"]["boundary"] = Value::String("SITE_INPUT".into());
        let bytes = serde_json::to_vec(&corpus).expect("wrong-boundary corpus");
        assert!(audit_corpus(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn uncertainty_intervals_must_contain_the_source_screen_value() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        corpus["candidates"][0]["battery_energy"]["lower"] = json!(100.0);
        corpus["candidates"][0]["battery_energy"]["upper"] = json!(101.0);
        let bytes = serde_json::to_vec(&corpus).expect("invalid uncertainty corpus");
        assert!(audit_corpus(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn mission_units_must_match_the_single_service_contract() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        let report = corpus["mission_reports"].as_array_mut().unwrap().iter_mut()
            .find(|report| report["report_ref"] == corpus["candidates"][0]["source_report_ref"])
            .unwrap();
        report["report"]["results"][0]["mission_functional_unit"] = Value::String("passenger-km per shift".into());
        let bytes = serde_json::to_vec(&corpus).expect("mixed service unit corpus");
        assert!(audit_corpus(SCHEMA, &bytes).is_err());
    }

    #[test]
    fn duplicate_source_scenario_cannot_be_counted_as_two_options() {
        let mut corpus: Corpus = serde_json::from_slice(CORPUS).expect("corpus");
        corpus.candidates[1].source_report_ref = corpus.candidates[0].source_report_ref.clone();
        corpus.candidates[1].scenario_id = corpus.candidates[0].scenario_id.clone();
        corpus.candidates[1].source_report_sha256 = corpus.candidates[0].source_report_sha256.clone();
        corpus.candidates[1].source_result_sha256 = corpus.candidates[0].source_result_sha256.clone();
        let bytes = serde_json::to_vec(&corpus).expect("duplicated scenario corpus");
        let error = audit_corpus(SCHEMA, &bytes).expect_err("source scenario cannot be reused");
        assert!(error.contains("reused by multiple alternatives"));
    }

    #[test]
    fn no_service_feasible_option_returns_an_empty_frontier() {
        let mut corpus: Value = serde_json::from_slice(CORPUS).expect("corpus");
        for candidate in corpus["candidates"].as_array_mut().unwrap() {
            candidate["service_delivery"]["lower"] = json!(0.0);
            candidate["service_delivery"]["upper"] = json!(0.0);
        }
        let bytes = serde_json::to_vec(&corpus).expect("no service feasible corpus");
        let audit = audit_corpus(SCHEMA, &bytes).expect("no-frontier result");
        assert_eq!(audit.eligible_candidate_count, 0);
        assert!(audit.robust_frontier_candidate_ids.is_empty());
    }
}
