// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only semantic evaluator for synthetic water-service restoration evidence.
//! It does not prove a physical repair, measure a real service outcome, or assess potable quality.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_ID: &str = "water-service-recovery-verification-v0.1";
const ORACLE_ID: &str = "water-service-recovery-disposition-oracle-v0.1";
const EXPECTED_CASES: usize = 13;
const REQUIRED_AREAS: [&str; 3] = ["DMA-A", "DMA-B", "DMA-C"];
const FAILURE_DOMAINS: [&str; 5] = [
    "physical_interface",
    "calibration_lineage",
    "power_supply",
    "communications_path",
    "clock_domain",
];

#[derive(Clone, Debug)]
struct Profile {
    required_area_ids: BTreeSet<String>,
    metric: String,
    unit: String,
    minimum_floor: f64,
    maximum_observation_age_seconds: u64,
    minimum_stable_dwell_seconds: u64,
    minimum_independent_paths: usize,
}

#[derive(Clone, Debug)]
struct EvidencePath {
    path_ref: String,
    source_family: String,
    evidence_state: String,
    failure_domains: BTreeSet<(String, String)>,
}

#[derive(Clone, Debug)]
struct AreaResult {
    area_id: String,
    disposition: String,
    independent_path_count: usize,
    independent_pair_count: usize,
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
}

fn get_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn required_string(value: &Value, key: &str, context: &str) -> Result<String, String> {
    match get_str(value, key) {
        Some(text) if !text.trim().is_empty() => Ok(text.to_owned()),
        _ => Err(format!("{context}: missing or invalid string field {key}")),
    }
}

fn required_array<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: {key} must be an array"))
}

fn required_bool(value: &Value, key: &str, context: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{context}: {key} must be a boolean"))
}

fn finite_number(value: &Value, key: &str, context: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{context}: {key} must be a finite number"))
}

fn nullable_number(value: &Value, key: &str, context: &str) -> Result<Option<f64>, String> {
    match value.get(key) {
        None => Err(format!("{context}: missing field {key}")),
        Some(item) if item.is_null() => Ok(None),
        Some(item) => match item.as_f64().filter(|number| number.is_finite()) {
            Some(number) => Ok(Some(number)),
            None => Err(format!("{context}: {key} must be finite or null")),
        },
    }
}

fn unsigned_integer(value: &Value, key: &str, context: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{context}: {key} must be an unsigned integer"))
}

fn nullable_unsigned_integer(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<Option<u64>, String> {
    match value.get(key) {
        None => Err(format!("{context}: missing field {key}")),
        Some(item) if item.is_null() => Ok(None),
        Some(item) => match item.as_u64() {
            Some(number) => Ok(Some(number)),
            None => Err(format!("{context}: {key} must be an unsigned integer or null")),
        },
    }
}

fn is_one_of(value: &str, values: &[&str]) -> bool {
    values.contains(&value)
}

fn parse_profile(scenarios: &Value) -> Result<Profile, String> {
    if get_str(scenarios, "schema_version") != Some("0.1.0")
        || get_str(scenarios, "corpus_id") != Some(CORPUS_ID)
    {
        return Err("scenario corpus identity/version mismatch".to_owned());
    }

    let profile = scenarios
        .get("profile")
        .ok_or_else(|| "scenario corpus is missing profile".to_owned())?;

    if get_str(profile, "profile_id") != Some("SYNTHETIC-RECOVERY-PROFILE-V0")
        || get_str(profile, "evidence_class") != Some("synthetic_design_profile")
        || get_str(profile, "metric") != Some("delivered_demand_fraction")
        || get_str(profile, "unit")
            != Some("fraction_of_required_volume_over_declared_window")
        || get_str(profile, "quality_assessment")
            != Some("not_in_scope_quality_must_be_qualified_separately")
        || profile.get("values_are_normative").and_then(Value::as_bool) != Some(false)
    {
        return Err("profile identity, metric, unit, quality boundary or non-normative marker changed".to_owned());
    }

    let area_values = required_array(profile, "required_area_ids", "profile")?;
    let required_area_ids: Vec<String> = area_values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("profile.required_area_ids[{index}] must be a non-empty string"))
        })
        .collect::<Result<_, _>>()?;
    let required_set: BTreeSet<String> = required_area_ids.iter().cloned().collect();
    let canonical_set: BTreeSet<String> = REQUIRED_AREAS.iter().map(|id| (*id).to_owned()).collect();
    if required_area_ids.len() != REQUIRED_AREAS.len()
        || required_set.len() != required_area_ids.len()
        || required_set != canonical_set
    {
        return Err("required area IDs must be exactly DMA-A, DMA-B and DMA-C in v0.1".to_owned());
    }

    let minimum_floor = finite_number(profile, "minimum_floor", "profile")?;
    if !(0.0..=1.0).contains(&minimum_floor) {
        return Err("profile.minimum_floor must be in [0, 1]".to_owned());
    }
    let maximum_observation_age_seconds =
        unsigned_integer(profile, "maximum_observation_age_seconds", "profile")?;
    let minimum_stable_dwell_seconds =
        unsigned_integer(profile, "minimum_stable_dwell_seconds", "profile")?;
    let minimum_independent_paths =
        unsigned_integer(profile, "minimum_independent_paths", "profile")? as usize;
    if maximum_observation_age_seconds == 0
        || minimum_stable_dwell_seconds == 0
        || minimum_independent_paths < 2
    {
        return Err("profile freshness, dwell or independent-path threshold is invalid".to_owned());
    }

    Ok(Profile {
        required_area_ids: required_set,
        metric: "delivered_demand_fraction".to_owned(),
        unit: "fraction_of_required_volume_over_declared_window".to_owned(),
        minimum_floor,
        maximum_observation_age_seconds,
        minimum_stable_dwell_seconds,
        minimum_independent_paths,
    })
}

fn validate_schema_contract(schema: &Value) -> Result<(), String> {
    if get_str(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema")
        || get_str(schema, "type") != Some("object")
        || schema.get("additionalProperties").and_then(Value::as_bool) != Some(false)
    {
        return Err("schema must declare a closed JSON Schema Draft 2020-12 object root".to_owned());
    }
    let root_required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for key in [
        "schema_version",
        "corpus_id",
        "profile",
        "scenarios",
        "interpretation_rules",
        "claim_ceiling",
    ] {
        if !root_required.contains(key) {
            return Err(format!("schema root must require {key}"));
        }
    }

    let scenario_required: BTreeSet<&str> = schema
        .get("properties")
        .and_then(|value| value.get("scenarios"))
        .and_then(|value| value.get("items"))
        .and_then(|value| value.get("required"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for key in [
        "scenario_id",
        "scenario_class",
        "event_ref",
        "event_lineage_ref",
        "areas",
        "repair_ticket",
        "alarm_status",
        "active_failed_domains",
        "unresolved_conflicting_evidence",
        "aggregate_diagnostic",
        "expected_overall_disposition",
    ] {
        if !scenario_required.contains(key) {
            return Err(format!("schema scenario must require {key}"));
        }
    }

    let area_required: BTreeSet<&str> = schema
        .get("properties")
        .and_then(|value| value.get("scenarios"))
        .and_then(|value| value.get("items"))
        .and_then(|value| value.get("properties"))
        .and_then(|value| value.get("areas"))
        .and_then(|value| value.get("items"))
        .and_then(|value| value.get("required"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for key in [
        "area_id",
        "floor_ref",
        "metric",
        "unit",
        "minimum_floor",
        "observation_lower_bound",
        "observation_upper_bound",
        "evidence_state",
        "coverage_numerator",
        "coverage_denominator",
        "observation_age_seconds",
        "area_binding_status",
        "evidence_paths",
        "stable_dwell_seconds",
        "diagnostic_weight",
        "expected_area_disposition",
    ] {
        if !area_required.contains(key) {
            return Err(format!("schema area must require {key}"));
        }
    }
    Ok(())
}

fn parse_active_failed_domains(
    scenario: &Value,
    context: &str,
) -> Result<BTreeSet<(String, String)>, String> {
    let rows = required_array(scenario, "active_failed_domains", context)?;
    let mut domains = BTreeSet::new();
    for row in rows {
        let kind = required_string(row, "domain_type", context)?;
        let id = required_string(row, "domain_id", context)?;
        if !FAILURE_DOMAINS.contains(&kind.as_str()) {
            return Err(format!("{context}: unsupported active failure domain {kind}"));
        }
        if !domains.insert((kind.clone(), id.clone())) {
            return Err(format!("{context}: duplicate active failed domain {kind}/{id}"));
        }
    }
    Ok(domains)
}

fn parse_evidence_paths(
    area: &Value,
    context: &str,
) -> Result<Vec<EvidencePath>, String> {
    let rows = required_array(area, "evidence_paths", context)?;
    let mut path_refs = BTreeSet::new();
    let mut paths = Vec::with_capacity(rows.len());

    for row in rows {
        let path_ref = required_string(row, "path_ref", context)?;
        let source_family = required_string(row, "source_family", context)?;
        let measurement_principle = required_string(row, "measurement_principle", context)?;
        if !is_one_of(
            &measurement_principle,
            &[
                "magnetic_flow_meter",
                "ultrasonic_flow_meter",
                "portable_pressure_logger",
                "customer_outlet_measurement",
            ],
        ) {
            return Err(format!("{context}: unsupported measurement principle"));
        }
        if !path_refs.insert(path_ref.clone()) {
            return Err(format!("{context}: duplicate evidence path reference {path_ref}"));
        }

        let evidence_state = required_string(row, "evidence_state", context)?;
        if !is_one_of(&evidence_state, &["current", "stale", "failed", "unverified"]) {
            return Err(format!("{context}/{path_ref}: unsupported path evidence_state"));
        }

        let dependency_rows = required_array(row, "failure_domains", context)?;
        let mut dependency_types = BTreeSet::new();
        let mut failure_domains = BTreeSet::new();
        for dependency in dependency_rows {
            let domain_type = required_string(dependency, "domain_type", context)?;
            let domain_id = required_string(dependency, "domain_id", context)?;
            if !FAILURE_DOMAINS.contains(&domain_type.as_str()) {
                return Err(format!("{context}/{path_ref}: unsupported failure domain {domain_type}"));
            }
            if !dependency_types.insert(domain_type.clone()) {
                return Err(format!("{context}/{path_ref}: duplicate failure-domain type {domain_type}"));
            }
            failure_domains.insert((domain_type, domain_id));
        }
        if dependency_types.len() != FAILURE_DOMAINS.len()
            || FAILURE_DOMAINS
                .iter()
                .any(|domain| !dependency_types.contains(*domain))
        {
            return Err(format!("{context}/{path_ref}: every path must declare all five failure domains exactly once"));
        }

        paths.push(EvidencePath {
            path_ref,
            source_family,
            evidence_state,
            failure_domains,
        });
    }
    Ok(paths)
}

fn count_independent_paths(
    paths: &[EvidencePath],
    active_failed_domains: &BTreeSet<(String, String)>,
) -> (usize, usize, Vec<String>) {
    let surviving: Vec<&EvidencePath> = paths
        .iter()
        .filter(|path| {
            path.evidence_state == "current"
                && !path
                    .failure_domains
                    .iter()
                    .any(|domain| active_failed_domains.contains(domain))
        })
        .collect();

    let mut independent_pair_count = 0usize;
    let mut participating_refs = BTreeSet::new();
    for left_index in 0..surviving.len() {
        for right_index in (left_index + 1)..surviving.len() {
            let left = surviving[left_index];
            let right = surviving[right_index];
            if left.source_family == right.source_family {
                continue;
            }
            if left
                .failure_domains
                .iter()
                .any(|domain| right.failure_domains.contains(domain))
            {
                continue;
            }
            independent_pair_count += 1;
            participating_refs.insert(left.path_ref.clone());
            participating_refs.insert(right.path_ref.clone());
        }
    }

    (
        surviving.len(),
        independent_pair_count,
        participating_refs.into_iter().collect(),
    )
}

fn area_disposition(
    area: &Value,
    profile: &Profile,
    active_failed_domains: &BTreeSet<(String, String)>,
    scenario_context: &str,
) -> Result<AreaResult, String> {
    let area_id = required_string(area, "area_id", scenario_context)?;
    let context = format!("{scenario_context}/{area_id}");

    if required_bool(area, "required", &context)? != true {
        return Err(format!("{context}: every profile area must remain required"));
    }

    let floor_ref = area
        .get("floor_ref")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty());
    let metric = required_string(area, "metric", &context)?;
    let unit = required_string(area, "unit", &context)?;
    let minimum_floor = area
        .get("minimum_floor")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite());
    if floor_ref.is_none()
        || metric != profile.metric
        || unit != profile.unit
        || !minimum_floor.is_some_and(|value| (value - profile.minimum_floor).abs() <= 1.0e-12)
    {
        return Ok(AreaResult {
            area_id,
            disposition: "invalid".to_owned(),
            independent_path_count: 0,
            independent_pair_count: 0,
            lower_bound: None,
            upper_bound: None,
        });
    }

    let evidence_state = required_string(area, "evidence_state", &context)?;
    if !is_one_of(
        &evidence_state,
        &["observed_current", "missing", "stale", "conflicting"],
    ) {
        return Err(format!("{context}: unsupported observation evidence state"));
    }

    let binding_status = required_string(area, "area_binding_status", &context)?;
    if !is_one_of(&binding_status, &["current", "stale", "unverified"]) {
        return Err(format!("{context}: unsupported service-area binding status"));
    }

    let coverage_numerator = unsigned_integer(area, "coverage_numerator", &context)?;
    let coverage_denominator =
        nullable_unsigned_integer(area, "coverage_denominator", &context)?;
    if coverage_denominator.is_some_and(|denominator| {
        denominator == 0 || coverage_numerator > denominator
    }) {
        return Err(format!("{context}: invalid coverage numerator/denominator"));
    }

    let observation_age = unsigned_integer(area, "observation_age_seconds", &context)?;
    let lower = nullable_number(area, "observation_lower_bound", &context)?;
    let upper = nullable_number(area, "observation_upper_bound", &context)?;
    for (label, value) in [("lower", lower), ("upper", upper)] {
        if value.is_some_and(|number| !(0.0..=1.0).contains(&number)) {
            return Err(format!("{context}: {label} bound must be within [0, 1]"));
        }
    }
    if matches!((lower, upper), (Some(lower), Some(upper)) if lower > upper) {
        return Err(format!("{context}: lower observation bound exceeds upper bound"));
    }

    let observation_ref = area
        .get("observation_ref")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty());
    let validity_ref = area
        .get("validity_window_ref")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty());
    let geometry_ref = required_string(area, "geometry_revision_ref", &context)?;
    let topology_ref = required_string(area, "topology_revision_ref", &context)?;
    let diagnostic_weight = finite_number(area, "diagnostic_weight", &context)?;
    if diagnostic_weight <= 0.0 {
        return Err(format!("{context}: diagnostic_weight must be positive"));
    }
    if geometry_ref.is_empty() || topology_ref.is_empty() {
        return Err(format!("{context}: geometry/topology revisions must be non-empty"));
    }

    let paths = parse_evidence_paths(area, &context)?;
    let (independent_path_count, independent_pair_count, _) =
        count_independent_paths(&paths, active_failed_domains);
    let stable_dwell = unsigned_integer(area, "stable_dwell_seconds", &context)?;

    let mut disposition = "indeterminate";
    if binding_status != "current"
        || evidence_state != "observed_current"
        || coverage_denominator.is_none()
        || coverage_denominator != Some(coverage_numerator)
        || observation_age > profile.maximum_observation_age_seconds
        || lower.is_none()
        || upper.is_none()
        || observation_ref.is_none()
        || validity_ref.is_none()
    {
        disposition = "indeterminate";
    } else if upper.is_some_and(|value| value < profile.minimum_floor) {
        disposition = "breached_floor";
    } else if lower.is_some_and(|value| value < profile.minimum_floor) {
        disposition = "indeterminate";
    } else if independent_pair_count == 0
        || independent_path_count < profile.minimum_independent_paths
        || stable_dwell < profile.minimum_stable_dwell_seconds
    {
        disposition = "candidate_restored";
    } else {
        disposition = "meets_floor";
    }

    Ok(AreaResult {
        area_id,
        disposition: disposition.to_owned(),
        independent_path_count,
        independent_pair_count,
        lower_bound: lower,
        upper_bound: upper,
    })
}

fn overall_disposition(
    area_dispositions: &[String],
    alarm_status: &str,
    unresolved_conflict: bool,
) -> String {
    if area_dispositions.iter().any(|state| state == "invalid") {
        return "invalid".to_owned();
    }
    let breach = area_dispositions.iter().any(|state| state == "breached_floor");
    let unknown = area_dispositions.iter().any(|state| state == "indeterminate");
    match (breach, unknown) {
        (true, true) => "breached_with_indeterminate".to_owned(),
        (true, false) => "not_restored".to_owned(),
        (false, true) => "indeterminate".to_owned(),
        (false, false) => {
            if area_dispositions.iter().any(|state| state == "candidate_restored")
                || alarm_status != "cleared"
                || unresolved_conflict
            {
                "candidate_restored".to_owned()
            } else {
                "verified_restored".to_owned()
            }
        }
    }
}

fn verify_aggregate(
    scenario: &Value,
    area_rows: &[Value],
    profile: &Profile,
    context: &str,
) -> Result<(), String> {
    let diagnostic = scenario
        .get("aggregate_diagnostic")
        .ok_or_else(|| format!("{context}: aggregate_diagnostic is missing"))?;
    if required_bool(diagnostic, "authoritative", context)? {
        return Err(format!("{context}: aggregate diagnostic must remain non-authoritative"));
    }
    let method = required_string(diagnostic, "method", context)?;
    match method.as_str() {
        "not_computable" => {
            if !diagnostic.get("value").is_some_and(Value::is_null) {
                return Err(format!("{context}: not_computable aggregate must have null value"));
            }
        }
        "weighted_average_by_synthetic_connection_count" => {
            let declared = finite_number(diagnostic, "value", context)?;
            let mut numerator = 0.0;
            let mut denominator = 0.0;
            for area in area_rows {
                let lower = area
                    .get("observation_lower_bound")
                    .and_then(Value::as_f64)
                    .filter(|value| value.is_finite());
                let weight = finite_number(area, "diagnostic_weight", context)?;
                let evidence_current =
                    get_str(area, "evidence_state") == Some("observed_current");
                let binding_current =
                    get_str(area, "area_binding_status") == Some("current");
                let complete_coverage = match (
                    area.get("coverage_numerator").and_then(Value::as_u64),
                    area.get("coverage_denominator").and_then(Value::as_u64),
                ) {
                    (Some(numerator), Some(denominator)) if denominator > 0 => {
                        numerator == denominator
                    }
                    _ => false,
                };
                if !evidence_current
                    || !binding_current
                    || !complete_coverage
                    || lower.is_none()
                    || weight <= 0.0
                    || unsigned_integer(area, "observation_age_seconds", context)?
                        > profile.maximum_observation_age_seconds
                {
                    return Err(format!(
                        "{context}: weighted diagnostic cannot use stale, incomplete, unbound or missing area evidence"
                    ));
                }
                numerator += lower.unwrap_or_default() * weight;
                denominator += weight;
            }
            if denominator <= 0.0 {
                return Err(format!("{context}: weighted diagnostic has no positive weight"));
            }
            let recomputed = numerator / denominator;
            if (recomputed - declared).abs() > 1.0e-8 {
                return Err(format!("{context}: weighted aggregate arithmetic mismatch"));
            }
        }
        _ => return Err(format!("{context}: unsupported aggregate diagnostic method")),
    }
    Ok(())
}

pub fn evaluate(
    scenarios: &Value,
    schema: &Value,
    oracle: &Value,
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    if let Err(error) = validate_schema_contract(schema) {
        errors.push(error);
    }
    if get_str(oracle, "schema_version") != Some("0.1.0")
        || get_str(oracle, "oracle_id") != Some(ORACLE_ID)
        || get_str(oracle, "corpus_id") != Some(CORPUS_ID)
    {
        errors.push("oracle identity/version mismatch".to_owned());
    }

    let profile = match parse_profile(scenarios) {
        Ok(profile) => Some(profile),
        Err(error) => {
            errors.push(error);
            None
        }
    };
    let Some(profile) = profile else {
        return Err(errors);
    };

    let cases = match scenarios.get("scenarios").and_then(Value::as_array) {
        Some(cases) => cases,
        None => return Err(vec!["scenario corpus.scenarios must be an array".to_owned()]),
    };
    let oracle_rows = match oracle.get("results").and_then(Value::as_array) {
        Some(rows) => rows,
        None => return Err(vec!["oracle.results must be an array".to_owned()]),
    };
    if cases.len() != EXPECTED_CASES || oracle_rows.len() != EXPECTED_CASES {
        errors.push(format!(
            "expected {EXPECTED_CASES} scenarios and oracle rows, found {} and {}",
            cases.len(),
            oracle_rows.len()
        ));
    }

    let mut oracle_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for row in oracle_rows {
        let Some(id) = get_str(row, "scenario_id") else {
            errors.push("oracle row missing scenario_id".to_owned());
            continue;
        };
        if oracle_by_id.insert(id.to_owned(), row).is_some() {
            errors.push(format!("duplicate oracle scenario_id {id}"));
        }
        if row.get("repair_ticket_is_restoration_evidence").and_then(Value::as_bool)
            != Some(false)
            || row.get("alarm_clearance_is_restoration_evidence").and_then(Value::as_bool)
                != Some(false)
            || get_str(row, "quality_assessment") != Some("not_in_scope")
            || get_str(row, "evidence_ceiling") != Some("synthetic_design_only")
        {
            errors.push(format!("{id}: oracle authority/evidence ceiling changed"));
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut output_results = Vec::new();
    for case in cases {
        let scenario_id = match get_str(case, "scenario_id") {
            Some(id) => id,
            None => {
                errors.push("scenario missing scenario_id".to_owned());
                continue;
            }
        };
        let context = scenario_id.to_owned();
        if !scenario_id.starts_with("WRV-") || !scenario_ids.insert(scenario_id.to_owned()) {
            errors.push(format!("{context}: invalid or duplicate scenario ID"));
        }
        let scenario_class = required_string(case, "scenario_class", &context);
        if scenario_class.is_err() {
            errors.push(scenario_class.err().unwrap_or_default());
        }
        if required_string(case, "event_ref", &context).is_err()
            || required_string(case, "event_lineage_ref", &context).is_err()
        {
            errors.push(format!("{context}: event identity/lineage is required"));
        }
        let ticket = case.get("repair_ticket").unwrap_or(&Value::Null);
        let ticket_status = required_string(ticket, "status", &context);
        if ticket_status
            .as_deref()
            .is_ok_and(|status| !is_one_of(status, &["open", "closed", "unknown"]))
        {
            errors.push(format!("{context}: unsupported repair ticket status"));
        }
        if required_string(ticket, "ticket_ref", &context).is_err() {
            errors.push(format!("{context}: repair ticket reference is required"));
        }
        let alarm_status = required_string(case, "alarm_status", &context)
            .unwrap_or_else(|error| {
                errors.push(error);
                "unknown".to_owned()
            });
        if !is_one_of(&alarm_status, &["active", "cleared", "unknown"]) {
            errors.push(format!("{context}: unsupported alarm status"));
        }
        let unresolved_conflict = required_bool(case, "unresolved_conflicting_evidence", &context)
            .unwrap_or_else(|error| {
                errors.push(error);
                true
            });

        let failed_domains = match parse_active_failed_domains(case, &context) {
            Ok(domains) => domains,
            Err(error) => {
                errors.push(error);
                BTreeSet::new()
            }
        };
        let areas = match case.get("areas").and_then(Value::as_array) {
            Some(areas) => areas,
            None => {
                errors.push(format!("{context}: areas must be an array"));
                continue;
            }
        };
        if areas.len() != profile.required_area_ids.len() {
            errors.push(format!(
                "{context}: exactly {} required area rows must be present",
                profile.required_area_ids.len()
            ));
        }

        let mut seen_area_ids = BTreeSet::new();
        let mut area_results = Vec::new();
        for area in areas {
            let area_id = match get_str(area, "area_id") {
                Some(id) => id.to_owned(),
                None => {
                    errors.push(format!("{context}: area_id missing"));
                    continue;
                }
            };
            if !seen_area_ids.insert(area_id.clone()) {
                errors.push(format!("{context}: duplicate area_id {area_id}"));
            }
            if !profile.required_area_ids.contains(&area_id) {
                errors.push(format!("{context}: unexpected area_id {area_id}"));
            }
            match area_disposition(area, &profile, &failed_domains, &context) {
                Ok(result) => area_results.push(result),
                Err(error) => errors.push(error),
            }
        }
        if seen_area_ids != profile.required_area_ids {
            errors.push(format!("{context}: required area ID set is incomplete or different"));
        }

        let geometry_revisions: BTreeSet<String> = areas
            .iter()
            .filter_map(|area| get_str(area, "geometry_revision_ref").map(str::to_owned))
            .collect();
        let topology_revisions: BTreeSet<String> = areas
            .iter()
            .filter_map(|area| get_str(area, "topology_revision_ref").map(str::to_owned))
            .collect();
        if geometry_revisions.len() != 1 || topology_revisions.len() != 1 {
            errors.push(format!("{context}: service-area vector mixes geometry/topology revisions"));
        }

        if let Err(error) = verify_aggregate(case, areas, &profile, &context) {
            errors.push(error);
        }

        let area_dispositions: Vec<String> = area_results
            .iter()
            .map(|result| result.disposition.clone())
            .collect();
        let overall = overall_disposition(
            &area_dispositions,
            &alarm_status,
            unresolved_conflict,
        );
        let expected_overall = get_str(case, "expected_overall_disposition").unwrap_or("");
        if !is_one_of(
            expected_overall,
            &[
                "verified_restored",
                "candidate_restored",
                "not_restored",
                "breached_with_indeterminate",
                "indeterminate",
                "invalid",
            ],
        ) {
            errors.push(format!("{context}: unsupported expected overall disposition"));
        }
        if overall != expected_overall {
            errors.push(format!(
                "{context}: expected overall {expected_overall}, recomputed {overall}"
            ));
        }

        let oracle_row = oracle_by_id.get(scenario_id).copied();
        if oracle_row.is_none() {
            errors.push(format!("{context}: scenario missing from separate oracle"));
        } else if let Some(row) = oracle_row {
            if get_str(row, "expected_overall_disposition") != Some(overall.as_str()) {
                errors.push(format!("{context}: overall result differs from separate oracle"));
            }
            let expected_areas = row
                .get("expected_area_dispositions")
                .and_then(Value::as_array);
            let mut expected_by_id = BTreeMap::new();
            if let Some(rows) = expected_areas {
                for expected in rows {
                    let area_id = get_str(expected, "area_id");
                    let disposition = get_str(expected, "disposition");
                    if let (Some(area_id), Some(disposition)) = (area_id, disposition) {
                        if expected_by_id
                            .insert(area_id.to_owned(), disposition.to_owned())
                            .is_some()
                        {
                            errors.push(format!("{context}: duplicate oracle area {area_id}"));
                        }
                    } else {
                        errors.push(format!("{context}: malformed oracle area disposition"));
                    }
                }
            } else {
                errors.push(format!("{context}: oracle expected_area_dispositions must be an array"));
            }
            if expected_by_id.len() != profile.required_area_ids.len() {
                errors.push(format!("{context}: oracle must contain exactly three area rows"));
            }
            for result in &area_results {
                if expected_by_id.get(&result.area_id) != Some(&result.disposition) {
                    errors.push(format!(
                        "{context}/{}: area result differs from separate oracle",
                        result.area_id
                    ));
                }
            }
        }

        output_results.push(json!({
            "scenario_id": scenario_id,
            "verification_state": "Matched",
            "overall_disposition": overall,
            "repair_ticket_status": ticket_status.ok(),
            "alarm_status": alarm_status,
            "area_results": area_results.iter().map(|result| json!({
                "area_id": result.area_id,
                "disposition": result.disposition,
                "independent_surviving_path_count": result.independent_path_count,
                "independent_pair_count": result.independent_pair_count,
                "lower_bound": result.lower_bound,
                "upper_bound": result.upper_bound
            })).collect::<Vec<_>>(),
            "aggregate_diagnostic_authoritative": false,
            "quality_assessment": "not_in_scope",
            "evidence_ceiling": "synthetic_design_only"
        }));
    }

    if scenario_ids.len() != EXPECTED_CASES
        || scenario_ids.len() != oracle_by_id.len()
        || oracle_by_id.keys().any(|id| !scenario_ids.contains(id))
    {
        errors.push("scenario and oracle case ID sets are not exactly equal".to_owned());
    }
    if !get_str(scenarios, "claim_ceiling")
        .is_some_and(|text| text.to_ascii_lowercase().contains("synthetic"))
    {
        errors.push("scenario claim ceiling must remain synthetic".to_owned());
    }

    if errors.is_empty() {
        Ok(json!({
            "schema_version": "water-service-recovery-evaluation-v0.1",
            "verification_state": "Matched",
            "qualification_status": "NotRun",
            "scenario_count": cases.len(),
            "matched_scenario_count": output_results.len(),
            "required_area_ids": REQUIRED_AREAS,
            "results": output_results,
            "evidence_class": "synthetic_restoration_semantics_only",
            "claim_ceiling": "restoration_state_semantics_only_no_physical_or_potable_quality_qualification",
            "nonclaims": [
                "No repair effectiveness, hydraulic trajectory or real community service is measured.",
                "No potable-water quality, public-health or regulatory result is established.",
                "Repair-ticket closure, alarm clearance and controller reset are not independent service proof.",
                "No pump/valve actuation, crew dispatch, rationing, allocation or relocation is authorized."
            ]
        }))
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIOS: &str =
        include_str!("../../../../research/water-service-engineering/recovery-verification/scenarios-v0.1.json");
    const SCHEMA: &str =
        include_str!("../../../../research/water-service-engineering/recovery-verification/scenarios-v0.1.schema.json");
    const ORACLE: &str =
        include_str!("../../../../research/water-service-engineering/recovery-verification/disposition-oracle-v0.1.json");

    fn fixtures() -> (Value, Value, Value) {
        (
            serde_json::from_str(SCENARIOS).expect("scenarios parse"),
            serde_json::from_str(SCHEMA).expect("schema parse"),
            serde_json::from_str(ORACLE).expect("oracle parse"),
        )
    }

    #[test]
    fn all_thirteen_cases_and_thirty_nine_area_states_match_oracle() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("canonical corpus validates");
        assert_eq!(result["scenario_count"], EXPECTED_CASES);
        assert_eq!(result["matched_scenario_count"], EXPECTED_CASES);
        assert_eq!(result["qualification_status"], "NotRun");
    }

    #[test]
    fn closed_work_order_does_not_prove_restoration() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][1]["repair_ticket_status"], "closed");
        assert_eq!(result["results"][1]["overall_disposition"], "not_restored");
    }

    #[test]
    fn weighted_average_cannot_hide_local_breach() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][4]["overall_disposition"], "not_restored");
        assert_eq!(result["results"][4]["aggregate_diagnostic_authoritative"], false);
    }

    #[test]
    fn single_independent_path_only_yields_candidate_restored() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][2]["area_results"][0]["disposition"], "candidate_restored");
    }

    #[test]
    fn shared_gateway_does_not_count_as_two_independent_paths() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][8]["area_results"][0]["independent_pair_count"], 0);
        assert_eq!(result["results"][8]["area_results"][0]["disposition"], "candidate_restored");
    }

    #[test]
    fn incomplete_dwell_blocks_restored_area() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][3]["area_results"][0]["disposition"], "candidate_restored");
    }

    #[test]
    fn stale_observation_cannot_verify_restoration() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][5]["area_results"][1]["disposition"], "indeterminate");
    }

    #[test]
    fn missing_required_observation_cannot_verify_restoration() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][6]["area_results"][1]["disposition"], "indeterminate");
    }

    #[test]
    fn interval_crossing_floor_is_indeterminate() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][7]["area_results"][1]["disposition"], "indeterminate");
    }

    #[test]
    fn stale_service_area_binding_is_indeterminate() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][9]["area_results"][1]["disposition"], "indeterminate");
    }

    #[test]
    fn breach_and_unknown_are_both_preserved() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][10]["overall_disposition"], "breached_with_indeterminate");
    }

    #[test]
    fn active_alarm_blocks_verified_restoration() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][11]["overall_disposition"], "candidate_restored");
    }

    #[test]
    fn recurrence_cannot_be_suppressed_by_prior_restoration_claim() {
        let (scenarios, schema, oracle) = fixtures();
        let result = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(result["results"][12]["overall_disposition"], "not_restored");
    }

    #[test]
    fn duplicate_required_area_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"][2]["area_id"] = json!("DMA-B");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("duplicate area must fail");
        assert!(errors.iter().any(|error| error.contains("duplicate area_id")));
    }

    #[test]
    fn omitted_required_area_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"].as_array_mut().expect("areas").pop();
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("missing area must fail");
        assert!(errors.iter().any(|error| error.contains("required area ID set is incomplete")));
    }

    #[test]
    fn wrong_metric_unit_is_invalid() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"][0]["unit"] = json!("m3/day");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("wrong unit must fail");
        assert!(errors.iter().any(|error| error.contains("expected area disposition") || error.contains("invalid")));
    }

    #[test]
    fn inverted_interval_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"][0]["observation_lower_bound"] = json!(0.95);
        scenarios["scenarios"][0]["areas"][0]["observation_upper_bound"] = json!(0.70);
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("inverted interval must fail");
        assert!(errors.iter().any(|error| error.contains("lower observation bound exceeds upper bound")));
    }

    #[test]
    fn duplicated_failure_domain_type_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"][0]["evidence_paths"][0]["failure_domains"][4]["domain_type"] =
            json!("clock_domain");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("duplicate domain type must fail");
        assert!(errors.iter().any(|error| error.contains("duplicate failure-domain type")));
    }

    #[test]
    fn aggregate_arithmetic_is_recomputed() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][4]["aggregate_diagnostic"]["value"] = json!(0.9);
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("incorrect aggregate must fail");
        assert!(errors.iter().any(|error| error.contains("weighted aggregate arithmetic mismatch")));
    }

    #[test]
    fn aggregate_cannot_use_stale_evidence() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"][0]["evidence_state"] = json!("stale");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("stale aggregate must fail");
        assert!(errors.iter().any(|error| error.contains("weighted diagnostic cannot use stale")));
    }

    #[test]
    fn non_normative_profile_is_required() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["profile"]["values_are_normative"] = json!(true);
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("normative profile must fail");
        assert!(errors.iter().any(|error| error.contains("non-normative marker changed")));
    }

    #[test]
    fn modified_oracle_overall_is_rejected() {
        let (scenarios, schema, mut oracle) = fixtures();
        oracle["results"][0]["expected_overall_disposition"] = json!("candidate_restored");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("altered oracle must fail");
        assert!(errors.iter().any(|error| error.contains("overall result differs from separate oracle")));
    }

    #[test]
    fn shared_failed_domain_removes_path_from_independent_count() {
        let (mut scenarios, schema, oracle) = fixtures();
        let domain = scenarios["scenarios"][0]["areas"][0]["evidence_paths"][0]["failure_domains"][3].clone();
        scenarios["scenarios"][0]["active_failed_domains"] = json!([domain]);
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("failed shared domain must change result");
        assert!(errors.iter().any(|error| error.contains("area result differs from separate oracle")));
    }

    #[test]
    fn quality_assessment_remains_out_of_scope() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["profile"]["quality_assessment"] = json!("potable_water_verified");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("quality boundary change must fail");
        assert!(errors.iter().any(|error| error.contains("quality boundary")));
    }
}
