// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic weighted proportional-fair water-filling baseline for WPT.
//!
//! This is a single-interval supervisory baseline, not robust MPC or a
//! real-time power-electronics controller. Every embedded health receipt is
//! checked by wpt-health-dispatch-contract before it can influence allocation.
//! Hard authorization, PCC, stability and minimum-service constraints are
//! non-compensatory.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use wpt_health_dispatch_contract::{EligibilityState, RecommendedAction, evaluate_eligibility};

const NUMERICAL_TOLERANCE: f64 = 1e-9;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    let map = value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))?;
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be numeric"))?;
    if !number.is_finite() {
        return Err(fail(&format!("{path}.{key}"), "must be finite"));
    }
    Ok(number)
}

fn bool_field(value: &Value, key: &str, path: &str) -> Result<bool, String> {
    field(value, key, path)?
        .as_bool()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be boolean"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchState {
    Allocated,
    NoEligibleReceivers,
    InfeasibleServiceFloors,
    GlobalHardLimitFailure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReceiverAllocation {
    pub receiver_id: String,
    pub eligibility_state: String,
    pub requested_power_kw: f64,
    pub minimum_service_power_kw: f64,
    pub health_derated_capacity_kw: f64,
    pub allocated_power_kw: f64,
    pub service_ratio: f64,
    pub weight: f64,
    pub eligible_for_allocation: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DispatchReport {
    pub run_id: String,
    pub state: DispatchState,
    pub source_power_limit_kw: f64,
    pub total_allocated_power_kw: f64,
    pub total_requested_power_kw: f64,
    pub total_unmet_requested_power_kw: f64,
    pub eligible_receiver_count: usize,
    pub excluded_receiver_count: usize,
    /// Jain index over eligible receivers' delivered/requested ratios.
    pub jain_service_ratio_eligible: Option<f64>,
    /// Jain index over all receivers; excluded receivers count as zero.
    pub jain_service_ratio_all: Option<f64>,
    /// Sum_i weight_i * ln(allocated_i / requested_i).
    pub weighted_log_service_objective: Option<f64>,
    pub receivers: Vec<ReceiverAllocation>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct Candidate {
    receiver_id: String,
    requested: f64,
    minimum: f64,
    capacity: f64,
    weight: f64,
    eligible: bool,
    eligibility_state: String,
    reasons: Vec<String>,
}

fn receipt_identity_matches(
    receipt: &Value,
    asset_id: &str,
    segment_id: &str,
    route_id: &str,
) -> Result<bool, String> {
    let identity = field(receipt, "asset_identity", "$.health_receipt")?;
    let receipt_asset = string_field(identity, "asset_id", "$.health_receipt.asset_identity")?;
    let receipt_segment = string_field(identity, "segment_id", "$.health_receipt.asset_identity")?;
    let receipt_route = string_field(identity, "route_id", "$.health_receipt.asset_identity")?;
    Ok(receipt_asset == asset_id && receipt_segment == segment_id && receipt_route == route_id)
}

fn classify_candidate(receiver: &Value, index: usize) -> Result<Candidate, String> {
    let path = format!("$.receivers[{index}]");
    exact_keys(
        receiver,
        &[
            "receiver_id",
            "asset_id",
            "segment_id",
            "route_id",
            "requested_power_kw",
            "minimum_service_power_kw",
            "priority_weight",
            "urgency_weight",
            "risk_derating_factor",
            "health_receipt",
        ],
        &path,
    )?;

    let receiver_id = string_field(receiver, "receiver_id", &path)?.to_string();
    let asset_id = string_field(receiver, "asset_id", &path)?;
    let segment_id = string_field(receiver, "segment_id", &path)?;
    let route_id = string_field(receiver, "route_id", &path)?;
    let requested = number_field(receiver, "requested_power_kw", &path)?;
    let minimum = number_field(receiver, "minimum_service_power_kw", &path)?;
    let priority = number_field(receiver, "priority_weight", &path)?;
    let urgency = number_field(receiver, "urgency_weight", &path)?;
    let risk_derating = number_field(receiver, "risk_derating_factor", &path)?;

    if requested <= 0.0 || minimum < 0.0 || minimum > requested {
        return Err(fail(
            &path,
            "requested power must be positive and service floor must be within [0, requested]",
        ));
    }
    if priority <= 0.0 || urgency <= 0.0 {
        return Err(fail(&path, "priority and urgency weights must be positive"));
    }
    if !(0.0..=1.0).contains(&risk_derating) {
        return Err(fail(&path, "risk_derating_factor must be within [0, 1]"));
    }
    // Weight by requested service so equal priority/urgency yields equal
    // proportional service, not equal kW irrespective of request size.
    let weight = priority * urgency * requested;
    if !weight.is_finite() || weight <= 0.0 {
        return Err(fail(
            &path,
            "priority*urgency*requested-power weight overflowed or is non-positive",
        ));
    }

    let receipt = field(receiver, "health_receipt", &path)?;
    let report = evaluate_eligibility(receipt)?;
    let identity_matches = receipt_identity_matches(receipt, asset_id, segment_id, route_id)?;
    if !identity_matches {
        return Ok(Candidate {
            receiver_id,
            requested,
            minimum,
            capacity: 0.0,
            weight,
            eligible: false,
            eligibility_state: "NotEvaluable".into(),
            reasons: vec![
                "health receipt asset/segment/route identity does not match the dispatch request"
                    .into(),
            ],
        });
    }

    let (eligible, capacity, extra_reason) = match (report.state, report.recommended_action) {
        (EligibilityState::DecisionSupportEligible, RecommendedAction::MayInformAllocationOnly) => {
            (true, requested, None)
        }
        (EligibilityState::PolicyRiskExceeded, RecommendedAction::DerateOrReserveMaintenance) => {
            let capacity = requested * risk_derating;
            (
                capacity > 0.0,
                capacity,
                Some(format!(
                    "risk budget exceeded; supervisory capacity derated by {:.3}",
                    risk_derating
                )),
            )
        }
        (
            EligibilityState::PolicyRiskExceeded,
            RecommendedAction::ExcludeFromPlannedServiceHorizon,
        ) => (false, 0.0, None),
        _ => (false, 0.0, None),
    };

    let mut reasons = report.reasons;
    if let Some(reason) = extra_reason {
        reasons.push(reason);
    }
    Ok(Candidate {
        receiver_id,
        requested,
        minimum,
        capacity,
        weight,
        eligible,
        eligibility_state: format!("{:?}", report.state),
        reasons,
    })
}

/// Maximize sum_i w_i * ln(p_i / request_i), subject to:
/// sum_i p_i <= power_budget and minimum_i <= p_i <= capacity_i.
///
/// KKT gives p_i(lambda)=clamp(w_i/lambda, minimum_i, capacity_i).
/// Bounded deterministic bisection finds lambda for this single-interval
/// convex problem. The result is optimal to the stated numeric tolerance.
fn weighted_proportional_fair_water_fill(
    candidates: &[Candidate],
    power_budget: f64,
) -> Result<Vec<f64>, String> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    if !power_budget.is_finite() || power_budget < 0.0 {
        return Err("power budget must be finite and non-negative".into());
    }

    let total_capacity = candidates.iter().map(|item| item.capacity).sum::<f64>();
    if !total_capacity.is_finite() {
        return Err("total receiver capacity overflowed".into());
    }
    if power_budget + NUMERICAL_TOLERANCE >= total_capacity {
        return Ok(candidates.iter().map(|item| item.capacity).collect());
    }

    let allocation_at = |lambda: f64| {
        candidates
            .iter()
            .map(|item| (item.weight / lambda).clamp(item.minimum, item.capacity))
            .collect::<Vec<_>>()
    };
    let sum_at = |lambda: f64| allocation_at(lambda).iter().sum::<f64>();

    let mut low = 0.0_f64;
    let mut high = 1.0_f64;
    let mut bracketed = false;
    for _ in 0..1024 {
        let sum = sum_at(high);
        if !sum.is_finite() {
            return Err("weighted proportional-fair sum overflowed".into());
        }
        if sum <= power_budget {
            bracketed = true;
            break;
        }
        high *= 2.0;
        if !high.is_finite() {
            return Err("weighted proportional-fair search overflowed".into());
        }
    }
    if !bracketed {
        return Err("could not bracket proportional-fair multiplier".into());
    }

    for _ in 0..128 {
        let middle = low + (high - low) / 2.0;
        if middle == low || middle == high {
            break;
        }
        if sum_at(middle) > power_budget {
            low = middle;
        } else {
            high = middle;
        }
    }

    let allocation = allocation_at(high);
    let total = allocation.iter().sum::<f64>();
    if !total.is_finite() || total > power_budget + 1e-7 {
        return Err("proportional-fair allocation exceeds source power budget".into());
    }
    Ok(allocation)
}

fn jain_index(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum = values.iter().sum::<f64>();
    let square_sum = values.iter().map(|value| value * value).sum::<f64>();
    if square_sum <= 0.0 || !square_sum.is_finite() || !sum.is_finite() {
        return None;
    }
    Some(sum * sum / (values.len() as f64 * square_sum))
}

fn empty_report(
    run_id: String,
    state: DispatchState,
    source_limit: f64,
    candidates: &[Candidate],
    reason: &str,
) -> DispatchReport {
    let total_requested = candidates.iter().map(|item| item.requested).sum::<f64>();
    let receivers = candidates
        .iter()
        .map(|item| ReceiverAllocation {
            receiver_id: item.receiver_id.clone(),
            eligibility_state: item.eligibility_state.clone(),
            requested_power_kw: item.requested,
            minimum_service_power_kw: item.minimum,
            health_derated_capacity_kw: item.capacity,
            allocated_power_kw: 0.0,
            service_ratio: 0.0,
            weight: item.weight,
            eligible_for_allocation: false,
            reasons: {
                let mut reasons = item.reasons.clone();
                reasons.push(reason.to_string());
                reasons
            },
        })
        .collect::<Vec<_>>();
    DispatchReport {
        run_id,
        state,
        source_power_limit_kw: source_limit,
        total_allocated_power_kw: 0.0,
        total_requested_power_kw: total_requested,
        total_unmet_requested_power_kw: total_requested,
        eligible_receiver_count: 0,
        excluded_receiver_count: candidates.len(),
        jain_service_ratio_eligible: None,
        jain_service_ratio_all: None,
        weighted_log_service_objective: None,
        receivers,
        claim_ceiling: "Single-interval synthetic weighted proportional-fair baseline only. No robust-MPC, real-time, field, electrical-safety or standards claim.",
    }
}

/// Evaluate one static multi-receiver episode using health-qualified
/// weighted proportional-fair water filling.
pub fn evaluate_episode(episode: &Value) -> Result<DispatchReport, String> {
    exact_keys(
        episode,
        &[
            "schema_version",
            "run_id",
            "source_power_limit_kw",
            "grid_pcc_envelope_pass",
            "network_stability_pass",
            "receivers",
            "quality_flags",
        ],
        "$",
    )?;
    if string_field(episode, "schema_version", "$")? != "wpt-proportional-fair-dispatch-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let run_id = string_field(episode, "run_id", "$")?.to_string();

    let flags = field(episode, "quality_flags", "$")?
        .as_array()
        .ok_or_else(|| fail("$.quality_flags", "must be an array"))?;
    if flags.len() != 1 || flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 dispatch episodes must carry exactly SYNTHETIC_ONLY",
        ));
    }

    let source_limit = number_field(episode, "source_power_limit_kw", "$")?;
    if source_limit <= 0.0 {
        return Err(fail("$.source_power_limit_kw", "must be positive"));
    }
    let grid_pass = bool_field(episode, "grid_pcc_envelope_pass", "$")?;
    let stability_pass = bool_field(episode, "network_stability_pass", "$")?;

    let rows = field(episode, "receivers", "$")?
        .as_array()
        .ok_or_else(|| fail("$.receivers", "must be an array"))?;
    if rows.is_empty() {
        return Err(fail("$.receivers", "at least one receiver is required"));
    }
    let mut candidates = Vec::with_capacity(rows.len());
    let mut receiver_ids = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        let candidate = classify_candidate(row, index)?;
        if !receiver_ids.insert(candidate.receiver_id.clone()) {
            return Err(fail(
                &format!("$.receivers[{index}]"),
                format!("duplicate receiver_id {}", candidate.receiver_id),
            ));
        }
        candidates.push(candidate);
    }

    if !grid_pass || !stability_pass {
        return Ok(empty_report(
            run_id,
            DispatchState::GlobalHardLimitFailure,
            source_limit,
            &candidates,
            "global grid/PCC envelope or network-stability gate failed; no receiver may be energized",
        ));
    }

    let eligible = candidates
        .iter()
        .filter(|item| item.eligible)
        .cloned()
        .collect::<Vec<_>>();
    if eligible.is_empty() {
        return Ok(empty_report(
            run_id,
            DispatchState::NoEligibleReceivers,
            source_limit,
            &candidates,
            "no receiver has a health-qualified allocation receipt",
        ));
    }

    for item in &eligible {
        if item.capacity + NUMERICAL_TOLERANCE < item.minimum {
            return Ok(empty_report(
                run_id,
                DispatchState::InfeasibleServiceFloors,
                source_limit,
                &candidates,
                "health derating leaves an eligible receiver below its minimum service floor",
            ));
        }
    }

    let minimum_power = eligible.iter().map(|item| item.minimum).sum::<f64>();
    if minimum_power > source_limit + NUMERICAL_TOLERANCE {
        return Ok(empty_report(
            run_id,
            DispatchState::InfeasibleServiceFloors,
            source_limit,
            &candidates,
            "shared source budget is below the sum of eligible minimum service floors",
        ));
    }

    let allocations = weighted_proportional_fair_water_fill(&eligible, source_limit)?;
    let mut allocated_by_id = BTreeMap::<String, f64>::new();
    let mut eligible_ratios = Vec::with_capacity(eligible.len());
    let mut weighted_objective = 0.0_f64;
    for (item, allocated) in eligible.iter().zip(allocations.iter().copied()) {
        if allocated + NUMERICAL_TOLERANCE < item.minimum
            || allocated > item.capacity + NUMERICAL_TOLERANCE
        {
            return Err(format!(
                "receiver {} violates its minimum/capacity constraint",
                item.receiver_id
            ));
        }
        allocated_by_id.insert(item.receiver_id.clone(), allocated);
        let ratio = allocated / item.requested;
        eligible_ratios.push(ratio);
        if allocated <= 0.0 {
            return Err(format!(
                "eligible receiver {} received zero power under a positive service floor",
                item.receiver_id
            ));
        }
        weighted_objective += item.weight * ratio.ln();
    }

    let total_allocated = allocations.iter().sum::<f64>();
    if !total_allocated.is_finite() || total_allocated > source_limit + 1e-7 {
        return Err("total allocation is non-finite or exceeds source power limit".into());
    }

    let mut results = Vec::with_capacity(candidates.len());
    let mut all_ratios = Vec::with_capacity(candidates.len());
    let mut total_requested = 0.0;
    let mut total_unmet = 0.0;
    let mut excluded_count = 0usize;
    for item in &candidates {
        total_requested += item.requested;
        let allocated = allocated_by_id
            .get(&item.receiver_id)
            .copied()
            .unwrap_or(0.0);
        let ratio = allocated / item.requested;
        all_ratios.push(ratio);
        total_unmet += (item.requested - allocated).max(0.0);
        if !item.eligible {
            excluded_count += 1;
        }
        let mut reasons = item.reasons.clone();
        if item.eligible {
            reasons.push("allocated by weighted proportional-fair water filling".into());
        } else if reasons.is_empty() {
            reasons.push("excluded by health eligibility gate".into());
        }
        results.push(ReceiverAllocation {
            receiver_id: item.receiver_id.clone(),
            eligibility_state: item.eligibility_state.clone(),
            requested_power_kw: item.requested,
            minimum_service_power_kw: item.minimum,
            health_derated_capacity_kw: item.capacity,
            allocated_power_kw: allocated,
            service_ratio: ratio,
            weight: item.weight,
            eligible_for_allocation: item.eligible,
            reasons,
        });
    }

    Ok(DispatchReport {
        run_id,
        state: DispatchState::Allocated,
        source_power_limit_kw: source_limit,
        total_allocated_power_kw: total_allocated,
        total_requested_power_kw: total_requested,
        total_unmet_requested_power_kw: total_unmet,
        eligible_receiver_count: eligible.len(),
        excluded_receiver_count: excluded_count,
        jain_service_ratio_eligible: jain_index(&eligible_ratios),
        jain_service_ratio_all: jain_index(&all_ratios),
        weighted_log_service_objective: Some(weighted_objective),
        receivers: results,
        claim_ceiling: "Single-interval synthetic weighted proportional-fair baseline only. No robust-MPC, real-time, field, electrical-safety or standards claim.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-proportional-fair-dispatch-v0/dispatch-episode.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic dispatch fixture JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    fn promote_health_receipts_to_measured_style(episode: &mut Value) {
        for receiver in episode["receivers"].as_array_mut().unwrap() {
            let receipt = &mut receiver["health_receipt"];
            receipt["evidence_kind"] = Value::String("MEASURED".into());
            receipt["quality_flags"] = Value::Array(vec![]);
        }
    }

    fn allocation_for<'a>(report: &'a DispatchReport, receiver_id: &str) -> &'a ReceiverAllocation {
        report
            .receivers
            .iter()
            .find(|receiver| receiver.receiver_id == receiver_id)
            .unwrap()
    }

    #[test]
    fn synthetic_episode_never_receives_operational_allocation() {
        let report = evaluate_episode(&fixture()).unwrap();
        assert_eq!(report.state, DispatchState::NoEligibleReceivers);
        assert_eq!(report.total_allocated_power_kw, 0.0);
        assert_eq!(report.eligible_receiver_count, 0);
    }

    #[test]
    fn health_aware_pf_allocates_only_to_in_domain_receivers() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        let report = evaluate_episode(&episode).unwrap();
        assert_eq!(report.state, DispatchState::Allocated);
        assert_eq!(report.eligible_receiver_count, 2);
        assert_eq!(report.excluded_receiver_count, 1);
        assert!((report.total_allocated_power_kw - 8.0).abs() < 1e-7);
        assert!(report.total_allocated_power_kw <= 8.0 + 1e-7);
        assert_eq!(
            allocation_for(&report, "vehicle-03").allocated_power_kw,
            0.0
        );
        assert!(allocation_for(&report, "vehicle-02").health_derated_capacity_kw <= 5.0 + 1e-12);
        assert!(report.jain_service_ratio_eligible.is_some());
        assert!(report.weighted_log_service_objective.is_some());
    }

    #[test]
    fn weighted_pf_matches_kkt_water_filling_solution() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        let report = evaluate_episode(&episode).unwrap();
        assert!((allocation_for(&report, "vehicle-01").allocated_power_kw - 3.0).abs() < 1e-7);
        assert!((allocation_for(&report, "vehicle-02").allocated_power_kw - 5.0).abs() < 1e-7);
    }

    #[test]
    fn equal_priority_and_urgency_preserve_equal_service_ratios() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        episode["source_power_limit_kw"] = Value::from(9.0);
        episode["receivers"][1]["health_receipt"]["forecast"]["target_event_cif"] =
            Value::from(0.03);
        episode["receivers"][1]["health_receipt"]["forecast"]["event_free_survival"] =
            Value::from(0.91);
        episode["receivers"][1]["health_receipt"]["forecast"]["cause_specific_cumulative_incidence"]
            [0]["probability"] = Value::from(0.03);
        episode["receivers"][1]["health_receipt"]["forecast"]["cause_specific_cumulative_incidence"]
            [2]["probability"] = Value::from(0.02);
        episode["receivers"][1]["priority_weight"] = Value::from(1.0);
        episode["receivers"][1]["urgency_weight"] = Value::from(1.0);
        let report = evaluate_episode(&episode).unwrap();
        let first = allocation_for(&report, "vehicle-01").service_ratio;
        let second = allocation_for(&report, "vehicle-02").service_ratio;
        assert!((first - second).abs() < 1e-7);
        assert!((first - 0.5).abs() < 1e-7);
    }

    #[test]
    fn source_budget_below_minimum_service_floors_returns_no_dispatch() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        episode["source_power_limit_kw"] = Value::from(3.0);
        let report = evaluate_episode(&episode).unwrap();
        assert_eq!(report.state, DispatchState::InfeasibleServiceFloors);
        assert_eq!(report.total_allocated_power_kw, 0.0);
    }

    #[test]
    fn health_risk_derating_below_minimum_floor_fails_closed() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        episode["receivers"][1]["risk_derating_factor"] = Value::from(0.1);
        let report = evaluate_episode(&episode).unwrap();
        assert_eq!(report.state, DispatchState::InfeasibleServiceFloors);
        assert_eq!(report.total_allocated_power_kw, 0.0);
    }

    #[test]
    fn hard_risk_cap_excludes_receiver_but_preserves_other_service() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        episode["receivers"][1]["health_receipt"]["dispatch_policy"]["decision_mode"] =
            Value::String("HARD_RISK_CAP".into());
        let report = evaluate_episode(&episode).unwrap();
        assert_eq!(report.state, DispatchState::Allocated);
        assert_eq!(report.eligible_receiver_count, 1);
        assert_eq!(
            allocation_for(&report, "vehicle-02").allocated_power_kw,
            0.0
        );
        assert!(allocation_for(&report, "vehicle-01").allocated_power_kw > 0.0);
    }

    #[test]
    fn global_pcc_or_stability_failure_blocks_every_receiver() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        episode["grid_pcc_envelope_pass"] = Value::Bool(false);
        let report = evaluate_episode(&episode).unwrap();
        assert_eq!(report.state, DispatchState::GlobalHardLimitFailure);
        assert_eq!(report.total_allocated_power_kw, 0.0);
        assert!(
            report
                .receivers
                .iter()
                .all(|receiver| receiver.allocated_power_kw == 0.0)
        );
    }

    #[test]
    fn mismatched_health_receipt_identity_excludes_receiver() {
        let mut episode = fixture();
        promote_health_receipts_to_measured_style(&mut episode);
        episode["receivers"][0]["health_receipt"]["asset_identity"]["asset_id"] =
            Value::String("different-asset".into());
        let report = evaluate_episode(&episode).unwrap();
        assert_eq!(report.state, DispatchState::Allocated);
        assert_eq!(report.eligible_receiver_count, 1);
        assert_eq!(
            allocation_for(&report, "vehicle-01").allocated_power_kw,
            0.0
        );
    }

    #[test]
    fn duplicate_receiver_ids_are_rejected() {
        let mut episode = fixture();
        episode["receivers"][1]["receiver_id"] = Value::String("vehicle-01".into());
        assert!(
            evaluate_episode(&episode)
                .unwrap_err()
                .contains("duplicate receiver_id")
        );
    }

    #[test]
    fn zero_priority_or_urgency_is_rejected() {
        let mut episode = fixture();
        episode["receivers"][0]["priority_weight"] = Value::from(0.0);
        assert!(
            evaluate_episode(&episode)
                .unwrap_err()
                .contains("must be positive")
        );
    }

    #[test]
    fn source_power_budget_must_be_finite_and_positive() {
        let mut episode = fixture();
        episode["source_power_limit_kw"] = Value::from(0.0);
        assert!(
            evaluate_episode(&episode)
                .unwrap_err()
                .contains("must be positive")
        );
    }

    #[test]
    fn unknown_episode_properties_fail_closed() {
        let mut episode = fixture();
        episode["optimizer_shortcut"] = Value::Bool(true);
        assert!(
            evaluate_episode(&episode)
                .unwrap_err()
                .contains("unknown property")
        );
    }
}
