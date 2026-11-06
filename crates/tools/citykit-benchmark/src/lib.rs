//! Bounded, deterministic CityKit reference oracle.
//!
//! Standalone synthetic reference model for service capacity, resource
//! balance, workcell contention, and owner-native capability receipt
//! consistency. It intentionally does not implement generic AND/OR capability
//! closure: Symthaea CIV-BOOT owns that theorem. It also grants no engineering,
//! statutory, procurement, dispatch, or physical actuation authority.
//!
//! A feasible result means only that the declared synthetic case satisfies
//! the finite checks implemented here.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

const SYNTHETIC_CURRENT: &str = "synthetic_current";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Disposition {
    FeasibleForSyntheticCase,
    Unsupported,
    Unknown,
    Infeasible,
    #[test]
    fn duplicate_capability_receipts_never_leak_a_favorable_projection() {
        let receipt = serde_json::json!({
            "target_id":"panel-line",
            "status":"LocallyClosed",
            "operationally_reachable":true,
            "locally_closed":true,
            "evidence_state":"synthetic_current",
            "source_owner":"symthaea",
            "source_contract":"CIV-BOOT-002",
            "source_revision":"fixture-v0.1",
            "source_receipt_id":"synthetic:duplicate",
            "source_subject_digest":"synthetic-subject:DUPLICATE-RECEIPT-CONTROL",
            "scenario_id":"DUPLICATE-RECEIPT-CONTROL",
            "selected_route_id":"route-local",
            "reason_codes":[]
        });
        let case = serde_json::json!({
            "case_id":"DUPLICATE-RECEIPT-CONTROL",
            "input":{"capability_assessments":[receipt.clone(),receipt]}
        });
        let actual = evaluate_case(&case);
        assert_eq!(actual["disposition"], "Unsupported");
        assert_eq!(actual["unknown_evidence"], serde_json::json!(["panel-line"]));
        assert_eq!(actual["locally_closed_targets"], serde_json::json!([]));
        assert_eq!(actual["selected_routes"], serde_json::json!([]));
        assert!(actual["reason_codes"].as_array().unwrap()
            .iter().any(|value| value == "DUPLICATE_CAPABILITY_ASSESSMENT"));
    }

    #[test]
    fn mismatched_receipt_subject_digest_abstains_and_cannot_select_route() {
        let case = serde_json::json!({
            "case_id":"SUBJECT-DIGEST-CONTROL",
            "input":{"capability_assessments":[{
                "target_id":"panel-line",
                "status":"LocallyClosed",
                "operationally_reachable":true,
                "locally_closed":true,
                "evidence_state":"synthetic_current",
                "source_owner":"symthaea",
                "source_contract":"CIV-BOOT-002",
                "source_revision":"fixture-v0.1",
                "source_receipt_id":"synthetic:wrong-subject",
                "source_subject_digest":"synthetic-subject:OTHER-CASE",
                "scenario_id":"SUBJECT-DIGEST-CONTROL",
                "selected_route_id":"route-unbound",
                "reason_codes":[]
            }]}
        });
        let actual = evaluate_case(&case);
        assert_eq!(actual["disposition"], "Unknown");
        assert_eq!(actual["unknown_evidence"], serde_json::json!(["panel-line"]));
        assert_eq!(actual["locally_closed_targets"], serde_json::json!([]));
        assert_eq!(actual["selected_routes"], serde_json::json!([]));
        assert!(actual["reason_codes"].as_array().unwrap()
            .iter().any(|value| value == "CAPABILITY_RECEIPT_SUBJECT_DIGEST_MISMATCH"));
    }

    #[test]
    fn contradictory_receipt_status_cannot_project_selected_route() {
        let case = serde_json::json!({
            "case_id":"STATUS-CONTRADICTION-CONTROL",
            "input":{"capability_assessments":[{
                "target_id":"panel-line",
                "status":"LocallyClosed",
                "operationally_reachable":true,
                "locally_closed":false,
                "evidence_state":"synthetic_current",
                "source_owner":"symthaea",
                "source_contract":"CIV-BOOT-002",
                "source_revision":"fixture-v0.1",
                "source_receipt_id":"synthetic:contradiction",
                "source_subject_digest":"synthetic-subject:STATUS-CONTRADICTION-CONTROL",
                "scenario_id":"STATUS-CONTRADICTION-CONTROL",
                "selected_route_id":"route-must-not-leak",
                "reason_codes":[]
            }]}
        });
        let actual = evaluate_case(&case);
        assert_eq!(actual["disposition"], "Unsupported");
        assert_eq!(actual["selected_routes"], serde_json::json!([]));
        assert_eq!(actual["locally_closed_targets"], serde_json::json!([]));
    }

    #[test]
    fn workcell_resource_demands_must_match_capacity_units() {
        let case = serde_json::json!({
            "case_id":"WORKCELL-UNIT-CONTROL",
            "input":{
                "resource_capacities":[{"id":"cell","unit":"workcell","capacity":1}],
                "work_packages":[{
                    "id":"task",
                    "start_tick":0,
                    "end_tick":1,
                    "resource_demands":[{"resource_id":"cell","quantity":1,"unit":"m3"}]
                }]
            }
        });
        let actual = evaluate_case(&case);
        assert_eq!(actual["disposition"], "Unsupported");
        assert!(actual["reason_codes"].as_array().unwrap()
            .iter().any(|value| value == "WORK_PACKAGE_RESOURCE_DEMAND_UNIT_MISMATCH"));
    }

    #[test]
    fn work_package_permutation_preserves_normalized_reference_result() {
        let corpus = corpus();
        let mut case = corpus["cases"].as_array().unwrap().iter()
            .find(|value| value["case_id"] == "CK-WP-001")
            .expect("workcell contention case").clone();
        let baseline = evaluate_case(&case);
        case["input"]["work_packages"].as_array_mut().unwrap().reverse();
        for package in case["input"]["work_packages"].as_array_mut().unwrap() {
            package["resource_demands"].as_array_mut().unwrap().reverse();
        }
        let permuted = evaluate_case(&case);
        assert_eq!(baseline, permuted);
    }

}

impl Disposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::FeasibleForSyntheticCase => "FeasibleForSyntheticCase",
            Self::Unsupported => "Unsupported",
            Self::Unknown => "Unknown",
            Self::Infeasible => "Infeasible",
        }
    }
}

#[derive(Debug, Default)]
struct ResultAccumulator {
    disposition: Option<Disposition>,
    reason_codes: BTreeSet<String>,
    unmet_services: BTreeSet<String>,
    import_dependent_targets: BTreeSet<String>,
    unreachable_targets: BTreeSet<String>,
    unknown_evidence: BTreeSet<String>,
    locally_closed_targets: BTreeSet<String>,
    operationally_reachable_targets: BTreeSet<String>,
    selected_routes: BTreeSet<(String, String)>,
}

impl ResultAccumulator {
    fn record(&mut self, disposition: Disposition, reason: impl Into<String>) {
        self.disposition = Some(
            self.disposition
                .map_or(disposition, |previous| previous.max(disposition)),
        );
        let reason = reason.into();
        if !reason.is_empty() {
            self.reason_codes.insert(reason);
        }
    }

    fn unknown(&mut self, id: &str, reason: &str) {
        self.unknown_evidence.insert(id.to_owned());
        self.record(Disposition::Unknown, reason);
    }

    fn finish(self) -> Value {
        let selected_routes = self
            .selected_routes
            .into_iter()
            .map(|(target, route)| json!({"target": target, "route": route}))
            .collect::<Vec<_>>();

        json!({
            "disposition": self.disposition.unwrap_or(Disposition::FeasibleForSyntheticCase).as_str(),
            "reason_codes": self.reason_codes.into_iter().collect::<Vec<_>>(),
            "unmet_services": self.unmet_services.into_iter().collect::<Vec<_>>(),
            "import_dependent_targets": self.import_dependent_targets.into_iter().collect::<Vec<_>>(),
            "unreachable_targets": self.unreachable_targets.into_iter().collect::<Vec<_>>(),
            "unknown_evidence": self.unknown_evidence.into_iter().collect::<Vec<_>>(),
            "locally_closed_targets": self.locally_closed_targets.into_iter().collect::<Vec<_>>(),
            "operationally_reachable_targets": self.operationally_reachable_targets.into_iter().collect::<Vec<_>>(),
            "selected_routes": selected_routes,
        })
    }
}

fn items<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value.get(key).and_then(Value::as_array).map_or(&[], Vec::as_slice)
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn boolean(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn finite_number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn nonnegative(value: &Value, key: &str) -> Option<f64> {
    finite_number(value, key).filter(|v| *v >= 0.0)
}

fn evidence_is_current(value: &Value) -> bool {
    text(value, "evidence_state") == SYNTHETIC_CURRENT
}

fn add_receipt_reasons(result: &mut ResultAccumulator, receipt: &Value) {
    for reason in items(receipt, "reason_codes") {
        if let Some(code) = reason.as_str() {
            result.reason_codes.insert(code.to_owned());
        }
    }
}

fn first_receipt_reason<'a>(receipt: &'a Value, fallback: &'static str) -> &'a str {
    items(receipt, "reason_codes")
        .first()
        .and_then(Value::as_str)
        .unwrap_or(fallback)
}

/// Recompute one fixture from its input only. The `expected` field is never
/// read, so it can be used as an independent reference against a future
/// candidate evaluator.
pub fn evaluate_case(case: &Value) -> Value {
    let case_id = text(case, "case_id");
    let input = case.get("input").unwrap_or(&Value::Null);
    let mut result = ResultAccumulator::default();

    // Consume capability assessments from the owning Symthaea CIV-BOOT
    // contract. This is receipt validation/projection, not closure calculation.
    // Duplicate target receipts invalidate that target as a whole: do not let
    // whichever record happens to appear first leak a favorable projection.
    let receipts = items(input, "capability_assessments");
    let mut assessment_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for receipt in receipts {
        let target = text(receipt, "target_id");
        if !target.is_empty() {
            *assessment_counts.entry(target).or_insert(0) += 1;
        }
    }
    let duplicate_targets = assessment_counts
        .iter()
        .filter_map(|(target, count)| (*count > 1).then_some(*target))
        .collect::<BTreeSet<_>>();
    for target in &duplicate_targets {
        result.record(Disposition::Unsupported, "DUPLICATE_CAPABILITY_ASSESSMENT");
        result.unknown_evidence.insert((*target).to_owned());
    }

    let mut assessments: BTreeMap<&str, &Value> = BTreeMap::new();
    for receipt in receipts {
        let target = text(receipt, "target_id");
        if target.is_empty() {
            result.record(Disposition::Unsupported, "CAPABILITY_TARGET_ID_MISSING");
            continue;
        }
        if duplicate_targets.contains(target) {
            continue;
        }
        assessments.insert(target, receipt);

        if text(receipt, "scenario_id") != case_id {
            result.unknown(target, "CAPABILITY_RECEIPT_SUBJECT_MISMATCH");
            continue;
        }
        if text(receipt, "source_owner").is_empty()
            || text(receipt, "source_contract").is_empty()
            || text(receipt, "source_revision").is_empty()
            || text(receipt, "source_receipt_id").is_empty()
            || text(receipt, "source_subject_digest").is_empty()
        {
            result.unknown(target, "CAPABILITY_RECEIPT_IDENTITY_INCOMPLETE");
            continue;
        }
        let expected_subject_digest = format!("synthetic-subject:{case_id}");
        if text(receipt, "source_subject_digest") != expected_subject_digest {
            result.unknown(target, "CAPABILITY_RECEIPT_SUBJECT_DIGEST_MISMATCH");
            continue;
        }

        if !evidence_is_current(receipt) {
            let fallback = match text(receipt, "evidence_state") {
                "stale" => "CAPABILITY_RECEIPT_STALE",
                "revoked" => "CAPABILITY_RECEIPT_REVOKED",
                _ => "CAPABILITY_RECEIPT_EVIDENCE_UNKNOWN",
            };
            let supplied_reason = first_receipt_reason(receipt, fallback);
            result.unknown(target, supplied_reason);
            add_receipt_reasons(&mut result, receipt);
            continue;
        }

        let status = text(receipt, "status");
        let operational = boolean(receipt, "operationally_reachable");
        let locally_closed = boolean(receipt, "locally_closed");

        let mut status_consistent = false;
        match status {
            "LocallyClosed" if operational == Some(true) && locally_closed == Some(true) => {
                status_consistent = true;
                result.locally_closed_targets.insert(target.to_owned());
                result.operationally_reachable_targets.insert(target.to_owned());
            }
            "ImportDependent" if operational == Some(true) && locally_closed == Some(false) => {
                status_consistent = true;
                result.import_dependent_targets.insert(target.to_owned());
                result.operationally_reachable_targets.insert(target.to_owned());
            }
            "Unavailable" if operational == Some(false) && locally_closed == Some(false) => {
                result.unreachable_targets.insert(target.to_owned());
                let reason = first_receipt_reason(receipt, "REQUIRED_CAPABILITY_UNAVAILABLE");
                result.record(Disposition::Infeasible, reason);
                add_receipt_reasons(&mut result, receipt);
            }
            "Unknown" if operational.is_none() && locally_closed.is_none() => {
                let reason = first_receipt_reason(receipt, "CAPABILITY_ASSESSMENT_UNKNOWN");
                result.unknown(target, reason);
                add_receipt_reasons(&mut result, receipt);
            }
            "LocallyClosed" | "ImportDependent" | "Unavailable" | "Unknown" => {
                result.record(Disposition::Unsupported, "CAPABILITY_STATUS_CONTRADICTION");
                result.unknown_evidence.insert(target.to_owned());
            }
            _ => {
                result.record(Disposition::Unsupported, "CAPABILITY_STATUS_UNSUPPORTED");
                result.unknown_evidence.insert(target.to_owned());
            }
        }

        let route = text(receipt, "selected_route_id");
        if status_consistent && !route.is_empty() {
            result.selected_routes.insert((target.to_owned(), route.to_owned()));
        }
    }

    // Service capacity and owner-native capability dependencies.
    let mut service_ids = BTreeSet::new();
    for service in items(input, "services") {
        let id = text(service, "id");
        let unit = text(service, "unit");
        if id.is_empty() || unit.is_empty() {
            result.record(Disposition::Unsupported, "SERVICE_ID_OR_UNIT_MISSING");
            continue;
        }
        if !service_ids.insert(id) {
            result.record(Disposition::Unsupported, "DUPLICATE_SERVICE_ID");
        }

        if !evidence_is_current(service) {
            let reason = match text(service, "evidence_state") {
                "stale" => "SERVICE_CAPACITY_EVIDENCE_STALE",
                "revoked" => "SERVICE_CAPACITY_EVIDENCE_REVOKED",
                _ => "SERVICE_CAPACITY_EVIDENCE_UNKNOWN",
            };
            result.unknown(id, reason);
        } else {
            match (
                nonnegative(service, "peak_demand"),
                nonnegative(service, "declared_capacity"),
            ) {
                (Some(demand), Some(capacity)) if capacity < demand => {
                    result.unmet_services.insert(id.to_owned());
                    result.record(Disposition::Infeasible, "SERVICE_CAPACITY_BELOW_PEAK_DEMAND");
                }
                (Some(_), Some(_)) => {}
                _ => result.record(Disposition::Unsupported, "SERVICE_CAPACITY_QUANTITY_INVALID"),
            }
        }

        for required in items(service, "required_capabilities") {
            let Some(required_id) = required.as_str() else {
                result.record(Disposition::Unsupported, "REQUIRED_CAPABILITY_ID_INVALID");
                continue;
            };
            if !assessments.contains_key(required_id) {
                result.unknown(required_id, "REQUIRED_CAPABILITY_RECORD_MISSING");
            }
        }
    }

    // Explicit resource balance. No implicit unit conversion is attempted.
    let mut resource_ids = BTreeSet::new();
    for resource in items(input, "resources") {
        let id = text(resource, "id");
        let unit = text(resource, "unit");
        if id.is_empty() || unit.is_empty() {
            result.record(Disposition::Unsupported, "RESOURCE_ID_OR_UNIT_MISSING");
            continue;
        }
        if !resource_ids.insert(id) {
            result.record(Disposition::Unsupported, "DUPLICATE_RESOURCE_ID");
        }
        if !evidence_is_current(resource) {
            result.unknown(id, "RESOURCE_EVIDENCE_NOT_CURRENT");
            continue;
        }

        for key in ["allocated_unit", "reserve_unit"] {
            if resource.get(key).and_then(Value::as_str).is_some_and(|value| value != unit) {
                result.record(Disposition::Unsupported, "INCOMPATIBLE_RESOURCE_UNITS");
            }
        }

        match (
            nonnegative(resource, "available"),
            nonnegative(resource, "allocated"),
            nonnegative(resource, "reserve"),
        ) {
            (Some(available), Some(allocated), Some(reserve))
                if available + f64::EPSILON < allocated + reserve =>
            {
                result.record(Disposition::Infeasible, "RESOURCE_SUPPLY_OVERCOMMITTED");
            }
            (Some(_), Some(_), Some(_)) => {}
            _ => result.record(Disposition::Unsupported, "RESOURCE_QUANTITY_INVALID"),
        }
    }

    // Finite workcell capacities; shared demands use half-open time intervals.
    let mut capacities: BTreeMap<&str, f64> = BTreeMap::new();
    let mut capacity_units: BTreeMap<&str, &str> = BTreeMap::new();
    for capacity in items(input, "resource_capacities") {
        let id = text(capacity, "id");
        let unit = text(capacity, "unit");
        let Some(amount) = nonnegative(capacity, "capacity") else {
            result.record(Disposition::Unsupported, "RESOURCE_CAPACITY_INVALID");
            continue;
        };
        if id.is_empty() || unit.is_empty() {
            result.record(Disposition::Unsupported, "RESOURCE_CAPACITY_ID_OR_UNIT_MISSING");
            continue;
        }
        if capacities.insert(id, amount).is_some() {
            result.record(Disposition::Unsupported, "DUPLICATE_RESOURCE_CAPACITY_ID");
        }
        capacity_units.insert(id, unit);
    }

    let mut work_package_ids = BTreeSet::new();
    let mut events: BTreeMap<&str, BTreeMap<i64, f64>> = BTreeMap::new();
    for package in items(input, "work_packages") {
        let package_id = text(package, "id");
        if package_id.is_empty() || !work_package_ids.insert(package_id) {
            result.record(Disposition::Unsupported, "WORK_PACKAGE_ID_MISSING_OR_DUPLICATE");
        }
        let start = package.get("start_tick").and_then(Value::as_i64);
        let end = package.get("end_tick").and_then(Value::as_i64);
        let (Some(start), Some(end)) = (start, end) else {
            result.record(Disposition::Unsupported, "WORK_PACKAGE_INTERVAL_INVALID");
            continue;
        };
        if end <= start {
            result.record(Disposition::Unsupported, "WORK_PACKAGE_INTERVAL_INVALID");
            continue;
        }

        for demand in items(package, "resource_demands") {
            let resource_id = text(demand, "resource_id");
            let demand_unit = text(demand, "unit");
            let Some(quantity) = nonnegative(demand, "quantity") else {
                result.record(Disposition::Unsupported, "WORK_PACKAGE_RESOURCE_DEMAND_INVALID");
                continue;
            };
            if !capacities.contains_key(resource_id) {
                result.unknown(resource_id, "WORK_PACKAGE_RESOURCE_CAPACITY_MISSING");
                continue;
            }
            if demand_unit.is_empty() {
                result.record(Disposition::Unsupported, "WORK_PACKAGE_RESOURCE_DEMAND_UNIT_MISSING");
                continue;
            }
            if capacity_units.get(resource_id).copied() != Some(demand_unit) {
                result.record(Disposition::Unsupported, "WORK_PACKAGE_RESOURCE_DEMAND_UNIT_MISMATCH");
                continue;
            }
            let timeline = events.entry(resource_id).or_default();
            *timeline.entry(start).or_insert(0.0) += quantity;
            *timeline.entry(end).or_insert(0.0) -= quantity;
        }
    }

    // Combined deltas at the same tick make [start_tick, end_tick) semantics
    // independent of iteration order for back-to-back reservations.
    for (resource_id, timeline) in events {
        let Some(limit) = capacities.get(resource_id).copied() else {
            continue;
        };
        let mut occupied = 0.0;
        for delta in timeline.values() {
            occupied += delta;
            if occupied > limit + f64::EPSILON {
                result.record(Disposition::Infeasible, "SHARED_RESOURCE_CAPACITY_EXCEEDED");
                break;
            }
        }
    }

    result.finish()
}

/// Compare only fields frozen in a case's expected object. The oracle still
/// emits its complete result, including fields that a fixture does not assert.
pub fn matches_expected(actual: &Value, expected: &Value) -> bool {
    expected
        .as_object()
        .is_some_and(|fields| fields.iter().all(|(key, value)| actual.get(key) == Some(value)))
}

#[cfg(test)]
mod tests {
    use super::{evaluate_case, matches_expected};
    use serde_json::Value;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn corpus() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/research/citykit/CITYKIT_SYNTH_CORPUS_V0_1.json");
        let bytes = std::fs::read(path).expect("CityKit fixture corpus must exist");
        serde_json::from_slice(&bytes).expect("CityKit fixture corpus must be valid JSON")
    }

    #[test]
    fn corpus_has_unique_case_ids_and_synthetic_classification() {
        let corpus = corpus();
        assert_eq!(corpus["classification"], "synthetic_fixture_only");
        let cases = corpus["cases"].as_array().expect("cases array");
        assert_eq!(cases.len(), 18);
        let ids = cases.iter()
            .map(|case| case["case_id"].as_str().expect("case id").to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), cases.len());
    }

    #[test]
    fn independent_reference_oracle_matches_frozen_expected_fields() {
        let corpus = corpus();
        for case in corpus["cases"].as_array().expect("cases array") {
            let actual = evaluate_case(case);
            assert!(
                matches_expected(&actual, &case["expected"]),
                "case {} mismatch\nactual: {}\nexpected: {}",
                case["case_id"],
                serde_json::to_string_pretty(&actual).unwrap(),
                serde_json::to_string_pretty(&case["expected"]).unwrap()
            );
        }
    }

    #[test]
    fn imported_capability_is_operational_but_not_locally_closed() {
        let mut case = serde_json::json!({
            "case_id": "CAP-IMPORT-CONTROL",
            "input": {"capability_assessments": [{
                "target_id":"panel-line", "status":"ImportDependent",
                "operationally_reachable":true, "locally_closed":false,
                "evidence_state":"synthetic_current", "source_owner":"symthaea",
                "source_contract":"CIV-BOOT-002", "source_revision":"fixture-v0.1",
                "source_receipt_id":"synthetic:import-control",
                "source_subject_digest":"synthetic-subject:CAP-IMPORT-CONTROL",
                "scenario_id":"CAP-IMPORT-CONTROL", "reason_codes":[]
            }]}
        });
        let actual = evaluate_case(&case);
        assert_eq!(actual["disposition"], "FeasibleForSyntheticCase");
        assert_eq!(actual["import_dependent_targets"], serde_json::json!(["panel-line"]));
        assert_eq!(actual["locally_closed_targets"], serde_json::json!([]));
        case["input"]["capability_assessments"][0]["locally_closed"] = Value::Bool(true);
        let contradicted = evaluate_case(&case);
        assert_eq!(contradicted["disposition"], "Unsupported");
        assert!(contradicted["reason_codes"].as_array().unwrap()
            .iter().any(|value| value == "CAPABILITY_STATUS_CONTRADICTION"));
    }

    #[test]
    fn stale_capacity_evidence_abstains_even_when_declared_capacity_is_large() {
        let case = serde_json::json!({
            "case_id":"STALE-SERVICE-CONTROL",
            "input":{"services":[{
                "id":"clinic","unit":"visits/day","peak_demand":10,
                "declared_capacity":1000,"evidence_state":"stale",
                "required_capabilities":[]
            }]}
        });
        let actual = evaluate_case(&case);
        assert_eq!(actual["disposition"], "Unknown");
        assert_eq!(actual["unknown_evidence"], serde_json::json!(["clinic"]));
    }

    #[test]
    fn half_open_intervals_allow_back_to_back_use() {
        let case = serde_json::json!({
            "case_id":"HALF-OPEN-CONTROL",
            "input":{
                "resource_capacities":[{"id":"cell","unit":"workcell","capacity":1}],
                "work_packages":[
                    {"id":"a","start_tick":0,"end_tick":5,"resource_demands":[{"resource_id":"cell","quantity":1,"unit":"workcell"}]},
                    {"id":"b","start_tick":5,"end_tick":9,"resource_demands":[{"resource_id":"cell","quantity":1,"unit":"workcell"}]}
                ]
            }
        });
        assert_eq!(evaluate_case(&case)["disposition"], "FeasibleForSyntheticCase");
    }
}
