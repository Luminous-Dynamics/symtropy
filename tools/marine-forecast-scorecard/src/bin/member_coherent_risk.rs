use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-member-coherent-risk/0.1.0";
const MAX_INDEPENDENT_SCENARIOS: usize = 4_096;

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn valid_digest(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn cvar95(values: &[f64]) -> (f64, usize) {
    if values.is_empty() {
        return (0.0, 0);
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|left, right| right.total_cmp(left));
    let tail_count = ((ordered.len() as f64 * 0.05).ceil() as usize).max(1);
    (mean(&ordered[..tail_count]), tail_count)
}

fn expand_independent(
    edge_distributions: &[Vec<f64>],
    edge_index: usize,
    running_total: f64,
    totals: &mut Vec<f64>,
) {
    if edge_index == edge_distributions.len() {
        totals.push(running_total);
        return;
    }
    for loss in &edge_distributions[edge_index] {
        expand_independent(edge_distributions, edge_index + 1, running_total + loss, totals);
    }
}

#[derive(Clone)]
struct RouteMetrics {
    route_id: String,
    coherent_losses: Vec<f64>,
    coherent_cvar95: f64,
    coherent_tail_count: usize,
    independent_marginal_cvar95: f64,
    independent_scenario_count: usize,
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut blockers = Vec::<String>::new();

    if string_at(value, "schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value, "scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    if string_at(value, "claim_ceiling") != Some("member_coherent_route_risk_only") {
        errors.push("claim_ceiling must be member_coherent_route_risk_only".to_owned());
    }
    let record_status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&record_status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let forecast = value.get("forecast").unwrap_or(&Value::Null);
    for (field, label) in [
        ("artifact_sha256", "forecast artifact"),
        ("availability_receipt_sha256", "availability receipt"),
        ("member_set_sha256", "ensemble member set"),
    ] {
        if !valid_digest(string_at(forecast, field)) {
            errors.push(format!("{label} must have a valid SHA-256 digest"));
        }
    }
    if string_at(forecast, "source_id").is_none_or(str::is_empty)
        || string_at(forecast, "cycle_id").is_none_or(str::is_empty)
        || string_at(forecast, "availability_receipt_ref").is_none_or(str::is_empty)
    {
        errors.push("forecast source, cycle and availability receipt identity are required".to_owned());
    }
    match (
        parse_time(string_at(forecast, "forecast_issue_time")),
        parse_time(string_at(forecast, "first_available_at")),
        parse_time(string_at(forecast, "decision_cutoff")),
    ) {
        (Some(issue), Some(available), Some(cutoff)) if issue <= available && available <= cutoff => {}
        (Some(issue), Some(available), Some(_)) if available < issue => {
            errors.push("forecast first availability precedes its issue time".to_owned());
        }
        (Some(_), Some(_), Some(_)) => {
            blockers.push("forecast ensemble was first available after the decision cutoff".to_owned());
        }
        _ => errors.push("forecast issue, first_available_at and decision_cutoff must be RFC3339 timestamps".to_owned()),
    }

    let declared_members = forecast.get("members").and_then(Value::as_array);
    let mut member_digests = BTreeMap::<String, String>::new();
    if let Some(members) = declared_members {
        if members.len() < 2 {
            errors.push("at least two forecast members are required".to_owned());
        }
        for (index, member) in members.iter().enumerate() {
            let id = string_at(member, "member_id").unwrap_or("").to_owned();
            let digest = string_at(member, "artifact_sha256").unwrap_or("").to_owned();
            if id.is_empty() || !valid_digest(Some(&digest)) {
                errors.push(format!("forecast.members[{index}] has a missing ID or invalid digest"));
            } else if member_digests.insert(id.clone(), digest).is_some() {
                errors.push(format!("forecast member ID '{id}' is duplicated"));
            }
        }
    } else {
        errors.push("forecast.members must be an array".to_owned());
    }
    let expected_members = member_digests.keys().cloned().collect::<BTreeSet<_>>();
    let canonical_member_set = member_digests.iter()
        .map(|(member_id, digest)| format!("{member_id}:{digest}\n"))
        .collect::<String>();
    let computed_member_set_digest = Sha256::digest(canonical_member_set.as_bytes())
        .iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    if string_at(forecast, "member_set_sha256") != Some(computed_member_set_digest.as_str()) {
        errors.push("member_set_sha256 does not match the canonical sorted member identity set".to_owned());
    }

    let routes = value.get("routes").and_then(Value::as_array);
    if routes.is_none_or(Vec::is_empty) || routes.is_some_and(|items| items.len() < 2) {
        errors.push("at least two route candidates are required".to_owned());
    }
    let mut route_ids = BTreeSet::<String>::new();
    let mut metrics = Vec::<RouteMetrics>::new();
    if let Some(routes) = routes {
        for (route_index, route) in routes.iter().enumerate() {
            let route_id = string_at(route, "route_id").unwrap_or("").to_owned();
            if route_id.is_empty() || !route_ids.insert(route_id.clone()) {
                errors.push(format!("routes[{route_index}] has a missing or duplicated route_id"));
            }
            let edge_ids = route.get("edge_ids").and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>())
                .unwrap_or_default();
            let edge_set = edge_ids.iter().cloned().collect::<BTreeSet<_>>();
            if edge_ids.is_empty() || edge_set.len() != edge_ids.len() {
                errors.push(format!("route '{route_id}' requires a non-empty unique ordered edge_ids list"));
            }
            let route_members = route.get("members").and_then(Value::as_array);
            let Some(route_members) = route_members else {
                errors.push(format!("route '{route_id}' requires member path records"));
                continue;
            };
            let mut seen_members = BTreeSet::<String>::new();
            let mut coherent_losses = Vec::<f64>::new();
            let mut edge_distributions = vec![Vec::<f64>::new(); edge_ids.len()];
            let mut route_valid = true;
            for member in route_members {
                let member_id = string_at(member, "member_id").unwrap_or("").to_owned();
                if member_id.is_empty() || !seen_members.insert(member_id.clone()) {
                    errors.push(format!("route '{route_id}' has a missing or duplicated member ID"));
                    route_valid = false;
                }
                if !valid_digest(string_at(member, "path_result_sha256")) {
                    errors.push(format!("route '{route_id}' member '{member_id}' has an invalid path-result digest"));
                    route_valid = false;
                }
                let expected_source_digest = member_digests.get(&member_id).map(String::as_str);
                if !valid_digest(string_at(member, "source_member_sha256"))
                    || expected_source_digest != string_at(member, "source_member_sha256")
                {
                    errors.push(format!("route '{route_id}' member '{member_id}' is not bound to its declared forecast member artifact"));
                    route_valid = false;
                }
                let losses = member.get("edge_losses").and_then(Value::as_array);
                if losses.is_none_or(|items| items.len() != edge_ids.len()) {
                    errors.push(format!("route '{route_id}' member '{member_id}' edge-loss count must match ordered route edges"));
                    route_valid = false;
                    continue;
                }
                let losses = losses.unwrap();
                let mut total = 0.0;
                for (edge_index, (expected_edge, loss_record)) in edge_ids.iter().zip(losses.iter()).enumerate() {
                    let actual_edge = string_at(loss_record, "edge_id").unwrap_or("");
                    let loss = loss_record.get("loss_kwh_equivalent").and_then(Value::as_f64);
                    if actual_edge != expected_edge {
                        errors.push(format!("route '{route_id}' member '{member_id}' edge order/identity mismatch at index {edge_index}"));
                        route_valid = false;
                    }
                    match loss {
                        Some(value) if value.is_finite() && value >= 0.0 => {
                            total += value;
                            edge_distributions[edge_index].push(value);
                        }
                        _ => {
                            errors.push(format!("route '{route_id}' member '{member_id}' has invalid edge loss"));
                            route_valid = false;
                        }
                    }
                }
                if route_valid {
                    coherent_losses.push(total);
                }
            }
            if seen_members != expected_members {
                blockers.push(format!("route '{route_id}' ensemble member set differs from the forecast member set"));
                route_valid = false;
            }
            if route_valid && edge_distributions.iter().all(|values| values.len() == seen_members.len()) {
                let (coherent_cvar95, coherent_tail_count) = cvar95(&coherent_losses);
                let scenario_count = edge_distributions.iter().try_fold(1_usize, |count, values| count.checked_mul(values.len()));
                match scenario_count {
                    Some(count) if count <= MAX_INDEPENDENT_SCENARIOS => {
                        let mut independent_totals = Vec::with_capacity(count);
                        expand_independent(&edge_distributions, 0, 0.0, &mut independent_totals);
                        let (independent_marginal_cvar95, _) = cvar95(&independent_totals);
                        metrics.push(RouteMetrics {
                            route_id,
                            coherent_losses,
                            coherent_cvar95,
                            coherent_tail_count,
                            independent_marginal_cvar95,
                            independent_scenario_count: count,
                        });
                    }
                    _ => {
                        errors.push(format!("route '{route_id}' exceeds independent marginal scenario limit"));
                    }
                }
            }
        }
    }

    let best_coherent_route = metrics.iter().min_by(|left, right| left.coherent_cvar95.total_cmp(&right.coherent_cvar95));
    let best_independent_route = metrics.iter().min_by(|left, right| left.independent_marginal_cvar95.total_cmp(&right.independent_marginal_cvar95));
    let rankings_differ = best_coherent_route.zip(best_independent_route)
        .is_some_and(|(coherent, independent)| coherent.route_id != independent.route_id);

    let route_reports = metrics.iter().map(|route| json!({
        "route_id": route.route_id,
        "member_coherent_mission_losses_kwh_equivalent": route.coherent_losses,
        "member_coherent_cvar95_kwh_equivalent": route.coherent_cvar95,
        "member_coherent_tail_member_count": route.coherent_tail_count,
        "independent_edge_marginal_resample_cvar95_kwh_equivalent": route.independent_marginal_cvar95,
        "independent_edge_marginal_scenario_count": route.independent_scenario_count
    })).collect::<Vec<_>>();

    let disposition = if !errors.is_empty() {
        "INVALID"
    } else if !blockers.is_empty() {
        "BLOCKED"
    } else if record_status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "MEMBER_COHERENCE_AUDITED_READ_ONLY"
    };
    let digest = Sha256::digest(raw);
    let digest_hex = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    json!({
        "engine_version": ENGINE_VERSION,
        "scorecard_id": string_at(value, "scorecard_id"),
        "disposition": disposition,
        "input_sha256": digest_hex,
        "route_count": metrics.len(),
        "routes": route_reports,
        "best_route_by_member_coherent_cvar95": best_coherent_route.map(|route| route.route_id.as_str()),
        "best_route_by_independent_edge_marginal_resampling": best_independent_route.map(|route| route.route_id.as_str()),
        "route_ranking_changes_when_member_coherence_is_destroyed": rankings_differ,
        "findings": {"errors": errors, "blockers": blockers},
        "claim_ceiling": "synthetic_or_read_only_path_risk_calculation_not_navigation_authority",
        "note": "Member-coherent CVaR is computed across whole-route losses for each declared member. The independent-edge marginal comparator deliberately destroys cross-edge member dependence; it is a diagnostic negative control, not an accepted route-risk estimator. Small-ensemble CVaR is descriptive and not a calibrated probability guarantee."
    })
}

fn run() -> Result<(), String> {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments.len() != 3 || arguments[1] != "audit-member-risk" {
        return Err("Usage: member-coherent-risk audit-member-risk <fixture.json>".to_owned());
    }
    let raw = fs::read(&arguments[2]).map_err(|error| format!("{}: {error}", arguments[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", arguments[2]))?;
    let report = validate(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "MEMBER_COHERENCE_AUDITED_READ_ONLY") => Ok(()),
        Some(disposition) => Err(format!("member-risk audit disposition: {disposition}")),
        None => Err("member-risk audit report has no disposition".to_owned()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../../fixtures/synthetic-member-coherent-risk-0001.json"))
            .expect("synthetic member-coherence fixture parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw, value)
    }

    #[test]
    fn synthetic_control_changes_route_ranking_when_member_coherence_is_destroyed() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["best_route_by_member_coherent_cvar95"], "route-correlated-alternating");
        assert_eq!(report["best_route_by_independent_edge_marginal_resampling"], "route-correlated-high-low");
        assert_eq!(report["route_ranking_changes_when_member_coherence_is_destroyed"], true);
    }

    #[test]
    fn first_availability_cannot_precede_forecast_issue_time() {
        let mut value = fixture();
        value["forecast"]["first_available_at"] = json!("2025-02-28T23:50:00Z");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn member_set_digest_is_bound_to_sorted_member_identities() {
        let mut value = fixture();
        value["forecast"]["member_set_sha256"] = json!("e".repeat(64));
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn late_first_availability_blocks_member_risk_admission() {
        let mut value = fixture();
        value["forecast"]["first_available_at"] = json!("2025-03-01T00:40:00Z");
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn every_route_must_preserve_exact_forecast_member_set() {
        let mut value = fixture();
        value["routes"][1]["members"].as_array_mut().unwrap().pop();
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn route_leg_order_and_identity_must_match() {
        let mut value = fixture();
        value["routes"][0]["members"][0]["edge_losses"][1]["edge_id"] = json!("wrong-edge");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn member_source_digest_must_match_declared_forecast_artifact() {
        let mut value = fixture();
        value["routes"][0]["members"][0]["source_member_sha256"] = json!("f".repeat(64));
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn duplicate_member_identity_is_rejected() {
        let mut value = fixture();
        value["routes"][0]["members"][1]["member_id"] = value["routes"][0]["members"][0]["member_id"].clone();
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn negative_edge_loss_is_rejected() {
        let mut value = fixture();
        value["routes"][0]["members"][0]["edge_losses"][0]["loss_kwh_equivalent"] = json!(-1.0);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }
}
