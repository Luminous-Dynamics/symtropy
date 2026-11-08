// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Semantic composition checks for GroundAirBench episode manifests.
//!
//! This evaluator checks receipt joins and evidence dispositions. It is not a
//! physical model, scheduler, controller, safety interlock, or operating permit.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const BLOCKING_DISPOSITIONS: &[&str] = &[
    "AMBIGUOUS",
    "STALE",
    "OUT_OF_DOMAIN",
    "UNKNOWN",
    "REVOKED",
    "NOT_ADMITTED",
    "NOT_EVALUATED",
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Receipt {
    receipt_id: String,
    role: String,
    evidence_class: String,
    disposition: String,
    configuration_sha256: String,
    validity_start_ms: u64,
    validity_end_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpportunityEvaluation {
    pub opportunity_id: String,
    /// BLOCKED, SYNTHETIC_FIXTURE_ONLY, SIMULATED_ONLY,
    /// EVIDENCE_COMPLETE_FOR_BENCHMARK, or NOT_QUALIFIED_FOR_BENCHMARK.
    /// None of these values is an operational admission or permit.
    pub benchmark_disposition: String,
    pub blocking_reasons: Vec<String>,
    pub evidence_classes: Vec<String>,
    pub receipt_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_class: String,
    pub episode_horizon_ms: u64,
    pub opportunity_evaluations: Vec<OpportunityEvaluation>,
    pub receiver_count: usize,
    pub policy_count: usize,
    pub claim_ceiling: String,
}

impl EvaluationReport {
    pub fn as_json(&self) -> Value {
        let opportunities: Vec<Value> = self
            .opportunity_evaluations
            .iter()
            .map(|item| {
                json!({
                    "opportunity_id": item.opportunity_id,
                    "benchmark_disposition": item.benchmark_disposition,
                    "blocking_reasons": item.blocking_reasons,
                    "evidence_classes": item.evidence_classes,
                    "receipt_ids": item.receipt_ids,
                })
            })
            .collect();

        json!({
            "report_version": "ground-air-evaluation-v0",
            "case_id": self.case_id,
            "evidence_class": self.evidence_class,
            "episode_horizon_ms": self.episode_horizon_ms,
            "opportunity_evaluations": opportunities,
            "receiver_count": self.receiver_count,
            "policy_count": self.policy_count,
            "claim_ceiling": self.claim_ceiling,
            "operational_permit": false,
        })
    }
}

fn fail(path: &str, reason: &str) -> String {
    format!("{path}: {reason}")
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .ok_or_else(|| fail(path, &format!("missing property {key}")))
}

fn string<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| fail(path, &format!("{key} must be a non-empty string")))
}

fn unsigned(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(path, &format!("{key} must be a non-negative integer")))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| fail(path, "must be an array"))
}

fn strings(value: &Value, path: &str) -> Result<Vec<String>, String> {
    array(value, path)?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| fail(&format!("{path}[{index}]"), "must be a non-empty string"))
        })
        .collect()
}

fn validate_digest(value: &str, path: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(fail(path, "must be lowercase hexadecimal SHA-256"));
    }
    Ok(())
}

fn parse_receipts(
    frozen_inputs: &Value,
    horizon_ms: u64,
) -> Result<BTreeMap<String, Receipt>, String> {
    validate_digest(
        string(frozen_inputs, "scenario_sha256", "$.frozen_inputs")?,
        "$.frozen_inputs.scenario_sha256",
    )?;
    validate_digest(
        string(frozen_inputs, "source_catalogue_sha256", "$.frozen_inputs")?,
        "$.frozen_inputs.source_catalogue_sha256",
    )?;

    let rows = array(
        field(frozen_inputs, "upstream_receipts", "$.frozen_inputs")?,
        "$.frozen_inputs.upstream_receipts",
    )?;
    let mut receipts = BTreeMap::new();

    for (index, row) in rows.iter().enumerate() {
        let path = format!("$.frozen_inputs.upstream_receipts[{index}]");
        let receipt_id = string(row, "receipt_id", &path)?.to_owned();
        let role = string(row, "role", &path)?.to_owned();
        let evidence_class = string(row, "evidence_class", &path)?.to_owned();
        let disposition = string(row, "disposition", &path)?.to_owned();
        let sha256 = string(row, "sha256", &path)?;
        let configuration_sha256 = string(row, "configuration_sha256", &path)?;
        validate_digest(sha256, &format!("{path}.sha256"))?;
        validate_digest(
            configuration_sha256,
            &format!("{path}.configuration_sha256"),
        )?;

        let validity = field(row, "validity", &path)?;
        let validity_start_ms = unsigned(validity, "start_ms", &format!("{path}.validity"))?;
        let validity_end_ms = unsigned(validity, "end_ms", &format!("{path}.validity"))?;
        if validity_start_ms >= validity_end_ms {
            return Err(fail(
                &format!("{path}.validity"),
                "start_ms must be less than end_ms",
            ));
        }

        let receipt = Receipt {
            receipt_id: receipt_id.clone(),
            role,
            evidence_class,
            disposition,
            configuration_sha256: configuration_sha256.to_owned(),
            validity_start_ms,
            validity_end_ms,
        };

        if receipts.insert(receipt_id.clone(), receipt).is_some() {
            return Err(fail(&path, &format!("duplicate receipt_id {receipt_id}")));
        }
    }

    if receipts.is_empty() {
        return Err(fail(
            "$.frozen_inputs.upstream_receipts",
            "at least one upstream receipt is required for an executable fixture",
        ));
    }

    // Keep the horizon argument explicit here: receipt coverage is evaluated
    // against it per opportunity so the report can name the blocking receipt.
    let _ = horizon_ms;
    Ok(receipts)
}

fn validate_scenario(scenario: &Value) -> Result<(u64, u64), String> {
    let slot_duration_ms = unsigned(scenario, "slot_duration_ms", "$.scenario")?;
    let slot_count = unsigned(scenario, "slot_count", "$.scenario")?;
    if slot_duration_ms == 0 || slot_count == 0 {
        return Err(fail(
            "$.scenario",
            "slot_duration_ms and slot_count must both be positive",
        ));
    }
    let horizon_ms = slot_duration_ms
        .checked_mul(slot_count)
        .ok_or_else(|| fail("$.scenario", "episode horizon overflows u64"))?;
    Ok((slot_duration_ms, horizon_ms))
}

fn validate_receivers(
    receivers: &Value,
    opportunities: &BTreeSet<String>,
    slot_count: u64,
) -> Result<usize, String> {
    let rows = array(receivers, "$.receivers")?;
    if rows.is_empty() {
        return Err(fail("$.receivers", "at least one receiver is required"));
    }

    let mut seen = BTreeSet::new();
    for (index, receiver) in rows.iter().enumerate() {
        let path = format!("$.receivers[{index}]");
        let id = string(receiver, "receiver_id", &path)?;
        if !seen.insert(id.to_owned()) {
            return Err(fail(&path, &format!("duplicate receiver_id {id}")));
        }
        validate_digest(
            string(receiver, "receiver_sha256", &path)?,
            &format!("{path}.receiver_sha256"),
        )?;

        let requested = unsigned(receiver, "requested_energy_mwh", &path)?;
        let minimum = unsigned(receiver, "minimum_service_mwh", &path)?;
        let initial = unsigned(receiver, "initial_battery_energy_mwh", &path)?;
        let reserve = unsigned(receiver, "reserve_energy_mwh", &path)?;
        let deadline = unsigned(receiver, "deadline_slot", &path)?;

        if minimum > requested {
            return Err(fail(
                &path,
                "minimum_service_mwh cannot exceed requested_energy_mwh",
            ));
        }
        if reserve > initial {
            return Err(fail(
                &path,
                "reserve_energy_mwh cannot exceed initial_battery_energy_mwh",
            ));
        }
        if deadline >= slot_count {
            return Err(fail(
                &path,
                "deadline_slot must fall within the frozen episode",
            ));
        }

        let eligible = strings(
            field(receiver, "eligible_opportunity_ids", &path)?,
            &format!("{path}.eligible_opportunity_ids"),
        )?;
        if eligible.is_empty() {
            return Err(fail(
                &path,
                "receiver must reference at least one eligible opportunity",
            ));
        }
        for opportunity_id in eligible {
            if !opportunities.contains(&opportunity_id) {
                return Err(fail(
                    &path,
                    &format!("unknown eligible opportunity {opportunity_id}"),
                ));
            }
        }
    }
    Ok(rows.len())
}

fn validate_policies(policies: &Value) -> Result<usize, String> {
    let rows = array(policies, "$.candidate_policies")?;
    if rows.is_empty() {
        return Err(fail(
            "$.candidate_policies",
            "at least one candidate policy is required",
        ));
    }
    let mut seen = BTreeSet::new();
    for (index, policy) in rows.iter().enumerate() {
        let path = format!("$.candidate_policies[{index}]");
        let id = string(policy, "policy_id", &path)?;
        if !seen.insert(id.to_owned()) {
            return Err(fail(&path, &format!("duplicate policy_id {id}")));
        }
        validate_digest(
            string(policy, "artifact_sha256", &path)?,
            &format!("{path}.artifact_sha256"),
        )?;
        if unsigned(policy, "compute_budget_ms", &path)? == 0 {
            return Err(fail(&path, "compute_budget_ms must be greater than zero"));
        }
    }
    Ok(rows.len())
}

fn evaluate_opportunity(
    opportunity: &Value,
    index: usize,
    receipts: &BTreeMap<String, Receipt>,
    horizon_ms: u64,
    slot_count: u64,
) -> Result<OpportunityEvaluation, String> {
    let path = format!("$.opportunities[{index}]");
    let opportunity_id = string(opportunity, "opportunity_id", &path)?.to_owned();
    let field_sha256 = string(opportunity, "field_sha256", &path)?;
    validate_digest(field_sha256, &format!("{path}.field_sha256"))?;
    validate_digest(
        string(opportunity, "receiver_geometry_sha256", &path)?,
        &format!("{path}.receiver_geometry_sha256"),
    )?;

    let capacity = unsigned(opportunity, "capacity_mwh", &path)?;
    let _ = capacity; // Capacity is retained as an input; this validator does not dispatch energy.

    let slots = array(
        field(opportunity, "available_slots", &path)?,
        &format!("{path}.available_slots"),
    )?;
    if slots.is_empty() {
        return Err(fail(
            &format!("{path}.available_slots"),
            "at least one available slot is required",
        ));
    }
    for (slot_index, slot) in slots.iter().enumerate() {
        let value = slot.as_u64().ok_or_else(|| {
            fail(
                &format!("{path}.available_slots[{slot_index}]"),
                "slot must be a non-negative integer",
            )
        })?;
        if value >= slot_count {
            return Err(fail(
                &format!("{path}.available_slots[{slot_index}]"),
                "available slot falls outside the frozen episode",
            ));
        }
    }

    let receipt_ids = strings(
        field(opportunity, "receipt_ids", &path)?,
        &format!("{path}.receipt_ids"),
    )?;
    let required_roles: BTreeSet<String> = strings(
        field(opportunity, "required_receipt_roles", &path)?,
        &format!("{path}.required_receipt_roles"),
    )?
    .into_iter()
    .collect();
    if required_roles.is_empty() {
        return Err(fail(
            &format!("{path}.required_receipt_roles"),
            "at least one required receipt role is required",
        ));
    }

    let mut referenced_roles = BTreeSet::new();
    let mut blockers = BTreeSet::new();
    let mut evidence_classes = BTreeSet::new();
    let mut referenced_ids = BTreeSet::new();
    let mut has_matching_physical_envelope = false;
    let mut has_measured_evidence = false;
    let mut has_simulated_evidence = false;
    let mut has_synthetic_evidence = false;
    let mut all_referenced_dispositions_qualified = true;

    for receipt_id in &receipt_ids {
        if !referenced_ids.insert(receipt_id.clone()) {
            return Err(fail(
                &path,
                &format!("duplicate receipt reference {receipt_id}"),
            ));
        }
        let receipt = receipts.get(receipt_id).ok_or_else(|| {
            fail(
                &path,
                &format!("unresolved upstream receipt_id {receipt_id}"),
            )
        })?;

        referenced_roles.insert(receipt.role.clone());
        evidence_classes.insert(receipt.evidence_class.clone());

        if receipt.role == "PHYSICAL_ENVELOPE" && receipt.configuration_sha256 == field_sha256 {
            has_matching_physical_envelope = true;
        }
        if receipt.role == "OBSERVABILITY" && receipt.configuration_sha256 != field_sha256 {
            blockers.insert(format!("{receipt_id}:FIELD_CONFIGURATION_MISMATCH"));
        }

        if receipt.validity_start_ms > 0 || receipt.validity_end_ms < horizon_ms {
            blockers.insert(format!("{receipt_id}:VALIDITY_DOES_NOT_COVER_EPISODE"));
        }

        if BLOCKING_DISPOSITIONS.contains(&receipt.disposition.as_str()) {
            blockers.insert(format!("{receipt_id}:{}", receipt.disposition));
        }
        if receipt.disposition != "QUALIFIED_FOR_DECLARED_SCOPE" {
            all_referenced_dispositions_qualified = false;
        }

        match receipt.evidence_class.as_str() {
            "SYNTHETIC_ONLY" => has_synthetic_evidence = true,
            "SIMULATED" => has_simulated_evidence = true,
            "BENCH_MEASURED" | "FIELD_MEASURED" => has_measured_evidence = true,
            "INDEPENDENTLY_REVIEWED" => {}
            other => {
                return Err(fail(
                    &path,
                    &format!("unsupported evidence class {other} on receipt {receipt_id}"),
                ));
            }
        }
    }

    for role in required_roles {
        if !referenced_roles.contains(&role) {
            return Err(fail(
                &path,
                &format!("missing required receipt role {role}"),
            ));
        }
    }
    if !has_matching_physical_envelope {
        return Err(fail(
            &path,
            "no PHYSICAL_ENVELOPE receipt is bound to this field configuration digest",
        ));
    }

    let benchmark_disposition = if !blockers.is_empty() {
        "BLOCKED"
    } else if has_synthetic_evidence {
        "SYNTHETIC_FIXTURE_ONLY"
    } else if has_simulated_evidence {
        "SIMULATED_ONLY"
    } else if has_measured_evidence && all_referenced_dispositions_qualified {
        "EVIDENCE_COMPLETE_FOR_BENCHMARK"
    } else {
        "NOT_QUALIFIED_FOR_BENCHMARK"
    };

    Ok(OpportunityEvaluation {
        opportunity_id,
        benchmark_disposition: benchmark_disposition.to_owned(),
        blocking_reasons: blockers.into_iter().collect(),
        evidence_classes: evidence_classes.into_iter().collect(),
        receipt_ids: referenced_ids.into_iter().collect(),
    })
}

/// Evaluate a GroundAirBench fixture's semantic receipt composition.
///
/// Structural JSON Schema validation is a separate step. This function
/// validates referential integrity, role coverage, field binding, validity
/// intervals and fail-closed evidence dispositions.
pub fn evaluate_case(value: &Value) -> Result<EvaluationReport, String> {
    let case_id = string(value, "case_id", "$")?.to_owned();
    let evidence_class = string(value, "evidence_class", "$")?.to_owned();
    let case_state = string(value, "case_state", "$")?;
    if case_state != "FIXTURE" {
        return Err(fail(
            "$.case_state",
            "only executable FIXTURE manifests are evaluated; NOT_RUN is not a pass",
        ));
    }

    let scenario = field(value, "scenario", "$")?;
    let (slot_duration_ms, horizon_ms) = validate_scenario(scenario)?;
    let slot_count = unsigned(scenario, "slot_count", "$.scenario")?;
    let _ = slot_duration_ms;

    let receipts = parse_receipts(field(value, "frozen_inputs", "$")?, horizon_ms)?;

    let opportunities_value = array(field(value, "opportunities", "$")?, "$.opportunities")?;
    if opportunities_value.is_empty() {
        return Err(fail(
            "$.opportunities",
            "at least one opportunity is required",
        ));
    }

    let mut opportunity_ids = BTreeSet::new();
    for (index, opportunity) in opportunities_value.iter().enumerate() {
        let path = format!("$.opportunities[{index}]");
        let id = string(opportunity, "opportunity_id", &path)?;
        if !opportunity_ids.insert(id.to_owned()) {
            return Err(fail(&path, &format!("duplicate opportunity_id {id}")));
        }
    }

    let mut opportunity_evaluations = Vec::with_capacity(opportunities_value.len());
    for (index, opportunity) in opportunities_value.iter().enumerate() {
        opportunity_evaluations.push(evaluate_opportunity(
            opportunity,
            index,
            &receipts,
            horizon_ms,
            slot_count,
        )?);
    }
    opportunity_evaluations.sort_by(|left, right| left.opportunity_id.cmp(&right.opportunity_id));

    let receiver_count = validate_receivers(
        field(value, "receivers", "$")?,
        &opportunity_ids,
        slot_count,
    )?;
    let policy_count = validate_policies(field(value, "candidate_policies", "$")?)?;

    Ok(EvaluationReport {
        case_id,
        evidence_class,
        episode_horizon_ms: horizon_ms,
        opportunity_evaluations,
        receiver_count,
        policy_count,
        claim_ceiling: string(value, "claim_ceiling", "$")?.to_owned(),
    })
}

/// Parse and evaluate one JSON episode manifest.
pub fn evaluate_json(input: &str) -> Result<EvaluationReport, String> {
    let value: Value =
        serde_json::from_str(input).map_err(|error| format!("invalid JSON: {error}"))?;
    evaluate_case(&value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FIXTURE: &str =
        include_str!("../../../research/ground-air-bench-v0/ground_air_episode.synthetic.json");
    const NOT_RUN_TEMPLATE: &str =
        include_str!("../../../research/ground-air-bench-v0/ground_air_episode.NOT_RUN.template.json");

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("GroundAirBench fixture JSON")
    }

    fn opportunity<'a>(report: &'a EvaluationReport, id: &str) -> &'a OpportunityEvaluation {
        report
            .opportunity_evaluations
            .iter()
            .find(|item| item.opportunity_id == id)
            .expect("opportunity report")
    }

    #[test]
    fn synthetic_fixture_reports_ground_as_synthetic_and_ambiguous_rf_as_blocked() {
        let report = evaluate_json(FIXTURE).unwrap();
        assert_eq!(report.episode_horizon_ms, 2_000);
        assert_eq!(report.receiver_count, 2);
        assert_eq!(report.policy_count, 3);
        assert_eq!(
            opportunity(&report, "ground-ipt-dock").benchmark_disposition,
            "SYNTHETIC_FIXTURE_ONLY"
        );
        let air = opportunity(&report, "air-rf-hover");
        assert_eq!(air.benchmark_disposition, "BLOCKED");
        assert!(
            air.blocking_reasons
                .contains(&"synthetic-air-observability-ambiguous:AMBIGUOUS".to_owned())
        );
    }

    #[test]
    fn no_opportunity_is_ever_reported_as_an_operational_permit() {
        let report = evaluate_json(FIXTURE).unwrap().as_json();
        assert_eq!(report["operational_permit"], json!(false));
        for item in report["opportunity_evaluations"].as_array().unwrap() {
            assert_ne!(item["benchmark_disposition"], json!("ADMITTED"));
            assert_ne!(item["benchmark_disposition"], json!("POWER_PERMITTED"));
        }
    }

    #[test]
    fn duplicate_receipt_ids_are_rejected() {
        let mut value = fixture();
        let duplicate = value["frozen_inputs"]["upstream_receipts"][0].clone();
        value["frozen_inputs"]["upstream_receipts"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(
            evaluate_case(&value)
                .unwrap_err()
                .contains("duplicate receipt_id")
        );
    }

    #[test]
    fn unresolved_receipt_reference_is_rejected() {
        let mut value = fixture();
        value["opportunities"][0]["receipt_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!("missing-receipt"));
        assert!(
            evaluate_case(&value)
                .unwrap_err()
                .contains("unresolved upstream receipt_id")
        );
    }

    #[test]
    fn missing_required_receipt_role_is_rejected() {
        let mut value = fixture();
        value["opportunities"][1]["receipt_ids"] = json!([
            "synthetic-site-budget",
            "synthetic-air-envelope",
            "synthetic-air-observability-ambiguous",
            "synthetic-meters",
            "synthetic-air-session-admission",
            "synthetic-air-standards",
            "synthetic-air-mission-energy"
        ]);
        assert!(
            evaluate_case(&value)
                .unwrap_err()
                .contains("missing required receipt role AUTHORIZATION")
        );
    }

    #[test]
    fn physical_envelope_configuration_must_match_field_digest() {
        let mut value = fixture();
        value["frozen_inputs"]["upstream_receipts"][3]["configuration_sha256"] =
            json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee");
        assert!(
            evaluate_case(&value)
                .unwrap_err()
                .contains("no PHYSICAL_ENVELOPE receipt is bound")
        );
    }

    #[test]
    fn stale_receipt_is_a_non_compensatory_blocker() {
        let mut value = fixture();
        value["frozen_inputs"]["upstream_receipts"][4]["disposition"] = json!("STALE");
        let report = evaluate_case(&value).unwrap();
        let air = opportunity(&report, "air-rf-hover");
        assert_eq!(air.benchmark_disposition, "BLOCKED");
        assert!(
            air.blocking_reasons
                .contains(&"synthetic-air-observability-ambiguous:STALE".to_owned())
        );
    }

    #[test]
    fn validity_that_does_not_cover_full_episode_blocks_opportunity() {
        let mut value = fixture();
        value["frozen_inputs"]["upstream_receipts"][0]["validity"]["end_ms"] = json!(1_000);
        let report = evaluate_case(&value).unwrap();
        let ground = opportunity(&report, "ground-ipt-dock");
        assert_eq!(ground.benchmark_disposition, "BLOCKED");
        assert!(
            ground
                .blocking_reasons
                .contains(&"synthetic-site-budget:VALIDITY_DOES_NOT_COVER_EPISODE".to_owned())
        );
    }

    #[test]
    fn out_of_domain_and_revoked_receipts_block() {
        let mut value = fixture();
        value["frozen_inputs"]["upstream_receipts"][4]["disposition"] = json!("OUT_OF_DOMAIN");
        value["frozen_inputs"]["upstream_receipts"][10]["disposition"] = json!("REVOKED");
        let report = evaluate_case(&value).unwrap();
        assert_eq!(
            opportunity(&report, "air-rf-hover").benchmark_disposition,
            "BLOCKED"
        );
        assert_eq!(
            opportunity(&report, "ground-ipt-dock").benchmark_disposition,
            "BLOCKED"
        );
    }

    #[test]
    fn row_permutation_does_not_change_the_sorted_report() {
        let original = evaluate_case(&fixture()).unwrap();
        let mut reordered = fixture();
        reordered["opportunities"].as_array_mut().unwrap().reverse();
        reordered["frozen_inputs"]["upstream_receipts"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let replayed = evaluate_case(&reordered).unwrap();
        assert_eq!(original, replayed);
        assert_eq!(original.as_json(), replayed.as_json());
    }

    #[test]
    fn unknown_receiver_opportunity_is_rejected() {
        let mut value = fixture();
        value["receivers"][0]["eligible_opportunity_ids"] = json!(["not-a-real-opportunity"]);
        assert!(
            evaluate_case(&value)
                .unwrap_err()
                .contains("unknown eligible opportunity")
        );
    }

    #[test]
    fn deadline_outside_episode_is_rejected() {
        let mut value = fixture();
        value["receivers"][0]["deadline_slot"] = json!(2);
        assert!(
            evaluate_case(&value)
                .unwrap_err()
                .contains("deadline_slot must fall within")
        );
    }

    #[test]
    fn not_run_template_cannot_be_promoted_to_pass() {
        let mut value = fixture();
        value["case_state"] = json!("NOT_RUN");
        assert!(
            evaluate_case(&value)
                .unwrap_err()
                .contains("NOT_RUN is not a pass")
        );
    }

    #[test]
    fn checked_in_not_run_template_cannot_be_promoted_to_pass() {
        let value: Value = serde_json::from_str(NOT_RUN_TEMPLATE).unwrap();
        assert_eq!(value["case_state"], json!("NOT_RUN"));
        assert!(evaluate_case(&value)
            .unwrap_err()
            .contains("NOT_RUN is not a pass"));
    }

    #[test]
    fn evidence_classes_are_preserved_per_opportunity() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(
            opportunity(&report, "ground-ipt-dock").evidence_classes,
            vec!["SYNTHETIC_ONLY".to_owned()]
        );
    }
}
