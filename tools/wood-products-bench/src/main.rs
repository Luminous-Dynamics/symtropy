use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VALIDATOR_VERSION: &str = "0.1.0";
const REQUIRED_IDS: [&str; 6] = [
    "SAWING_DRY_MASS_CLOSURE",
    "STRUCTURAL_GRADE_GATE",
    "DRYING_QUALITY_FLOOR",
    "HARVEST_REGENERATION_GUARD",
    "CUSTODY_REPLAY_AND_CORRECTION",
    "LIFECYCLE_RECOVERY_PARETO",
];

type Check<T> = Result<T, String>;

fn object<'a>(v: &'a Value, at: &str) -> Check<&'a Map<String, Value>> {
    v.as_object().ok_or_else(|| format!("{at}: expected object"))
}

fn exact_keys(v: &Value, expected: &[&str], at: &str) -> Check<()> {
    let obj = object(v, at)?;
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = obj.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}

fn field<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(v, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn text(v: &Value, at: &str) -> Check<String> {
    v.as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}: expected string"))
}

fn u64_at(v: &Value, at: &str) -> Check<u64> {
    v.as_u64()
        .ok_or_else(|| format!("{at}: expected non-negative integer"))
}

fn bool_at(v: &Value, at: &str) -> Check<bool> {
    v.as_bool()
        .ok_or_else(|| format!("{at}: expected boolean"))
}

fn array<'a>(v: &'a Value, at: &str) -> Check<&'a Vec<Value>> {
    v.as_array()
        .ok_or_else(|| format!("{at}: expected array"))
}

fn req_text(v: &Value, key: &str, at: &str) -> Check<String> {
    text(field(v, key, at)?, &format!("{at}.{key}"))
}

fn req_u64(v: &Value, key: &str, at: &str) -> Check<u64> {
    u64_at(field(v, key, at)?, &format!("{at}.{key}"))
}

fn req_bool(v: &Value, key: &str, at: &str) -> Check<bool> {
    bool_at(field(v, key, at)?, &format!("{at}.{key}"))
}

fn add(a: u64, b: u64, at: &str) -> Check<u64> {
    a.checked_add(b)
        .ok_or_else(|| format!("{at}: integer overflow"))
}

fn note_arrays(v: &Value, at: &str) -> Check<()> {
    for key in ["expected_assertions", "negative_controls"] {
        let notes = array(field(v, key, at)?, &format!("{at}.{key}"))?;
        if notes.is_empty() || notes.iter().any(|n| n.as_str().is_none()) {
            return Err(format!("{at}.{key}: expected a non-empty array of strings"));
        }
    }
    Ok(())
}

fn sorted_unique_strings(v: &Value, at: &str) -> Check<Vec<String>> {
    let values = array(v, at)?
        .iter()
        .map(|value| text(value, at))
        .collect::<Check<Vec<_>>>()?;
    let mut sorted = values.clone();
    sorted.sort();
    sorted.dedup();
    if sorted.len() != values.len() {
        return Err(format!("{at}: duplicate IDs are not allowed"));
    }
    Ok(sorted)
}

fn require_same_ids(actual: &[String], expected: &[String], at: &str) -> Check<()> {
    let mut a = actual.to_vec();
    let mut e = expected.to_vec();
    a.sort();
    e.sort();
    if a != e {
        return Err(format!("{at}: IDs differ; actual={a:?}, expected={e:?}"));
    }
    Ok(())
}

fn validate_mass_closure(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[0];
    exact_keys(
        v,
        &[
            "id",
            "evidence_class",
            "input_dry_mass_g",
            "material_streams",
            "expected_assertions",
            "negative_controls",
        ],
        at,
    )?;
    note_arrays(v, at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong scenario identity or evidence class"));
    }
    let input = req_u64(v, "input_dry_mass_g", at)?;
    if input == 0 {
        return Err(format!("{at}: input dry mass must be positive"));
    }
    let streams = array(field(v, "material_streams", at)?, &format!("{at}.material_streams"))?;
    if streams.is_empty() {
        return Err(format!("{at}: at least one material stream is required"));
    }
    let mut stream_ids = BTreeSet::new();
    let mut total = 0_u64;
    for stream in streams {
        exact_keys(stream, &["stream_id", "dry_mass_g"], &format!("{at}.stream"))?;
        let id = req_text(stream, "stream_id", at)?;
        if !stream_ids.insert(id.clone()) {
            return Err(format!("{at}: duplicate stream ID {id}"));
        }
        total = add(total, req_u64(stream, "dry_mass_g", at)?, at)?;
    }
    if total != input {
        return Err(format!(
            "{at}: dry-mass imbalance: input={input}, accounted streams={total}"
        ));
    }
    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "metrics": {
            "input_dry_mass_g": input,
            "accounted_dry_mass_g": total,
            "stream_count": streams.len()
        }
    }))
}

fn validate_grade_gate(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[1];
    exact_keys(
        v,
        &[
            "id",
            "evidence_class",
            "application",
            "options",
            "expected_selected_option_id",
            "expected_assertions",
            "negative_controls",
        ],
        at,
    )?;
    note_arrays(v, at)?;
    if req_text(v, "id", at)? != at
        || req_text(v, "evidence_class", at)? != "synthetic"
        || req_text(v, "application", at)? != "structural"
    {
        return Err(format!("{at}: identity, evidence class or application is invalid"));
    }
    let options = array(field(v, "options", at)?, &format!("{at}.options"))?;
    if options.is_empty() {
        return Err(format!("{at}: options must not be empty"));
    }
    let mut ids = BTreeSet::new();
    let mut eligible: Vec<(String, u64)> = Vec::new();
    for option in options {
        exact_keys(
            option,
            &[
                "option_id",
                "synthetic_margin_units",
                "grade_evidence_status",
                "inspection_pass",
            ],
            &format!("{at}.option"),
        )?;
        let id = req_text(option, "option_id", at)?;
        if !ids.insert(id.clone()) {
            return Err(format!("{at}: duplicate option ID {id}"));
        }
        let margin = req_u64(option, "synthetic_margin_units", at)?;
        let evidence = req_text(option, "grade_evidence_status", at)?;
        if !["qualified", "missing", "unknown"].contains(&evidence.as_str()) {
            return Err(format!("{at}.{id}: unsupported grade evidence state {evidence}"));
        }
        let inspection_pass = req_bool(option, "inspection_pass", at)?;
        if evidence == "qualified" && inspection_pass {
            eligible.push((id, margin));
        }
    }
    eligible.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let selected = eligible
        .first()
        .ok_or_else(|| format!("{at}: no option passes the grade and inspection gates"))?;
    let expected = req_text(v, "expected_selected_option_id", at)?;
    if selected.0 != expected {
        return Err(format!(
            "{at}: expected selected option {expected}, independently recomputed {}",
            selected.0
        ));
    }
    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "metrics": {
            "option_count": options.len(),
            "eligible_option_count": eligible.len(),
            "selected_option_id": selected.0,
            "selected_synthetic_margin_units": selected.1
        }
    }))
}

fn validate_drying(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[2];
    exact_keys(
        v,
        &[
            "id",
            "evidence_class",
            "minimum_moisture_basis_points",
            "maximum_moisture_basis_points",
            "schedules",
            "expected_selected_schedule_id",
            "expected_assertions",
            "negative_controls",
        ],
        at,
    )?;
    note_arrays(v, at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong scenario identity or evidence class"));
    }
    let min = req_u64(v, "minimum_moisture_basis_points", at)?;
    let max = req_u64(v, "maximum_moisture_basis_points", at)?;
    if min > max || max > 10_000 {
        return Err(format!("{at}: invalid moisture window"));
    }
    let schedules = array(field(v, "schedules", at)?, &format!("{at}.schedules"))?;
    if schedules.is_empty() {
        return Err(format!("{at}: schedules must not be empty"));
    }
    let mut ids = BTreeSet::new();
    let mut eligible: Vec<(String, u64)> = Vec::new();
    for schedule in schedules {
        exact_keys(
            schedule,
            &[
                "schedule_id",
                "energy_units",
                "final_moisture_basis_points",
                "moisture_evidence_status",
                "defect_detected",
            ],
            &format!("{at}.schedule"),
        )?;
        let id = req_text(schedule, "schedule_id", at)?;
        if !ids.insert(id.clone()) {
            return Err(format!("{at}: duplicate schedule ID {id}"));
        }
        let energy = req_u64(schedule, "energy_units", at)?;
        let evidence = req_text(schedule, "moisture_evidence_status", at)?;
        let moisture_value = field(schedule, "final_moisture_basis_points", at)?;
        let moisture = if moisture_value.is_null() {
            None
        } else {
            Some(u64_at(
                moisture_value,
                &format!("{at}.{id}.final_moisture_basis_points"),
            )?)
        };
        match (evidence.as_str(), moisture) {
            ("measured", Some(_)) | ("unknown", None) => {}
            _ => {
                return Err(format!(
                    "{at}.{id}: moisture value and evidence status contradict each other"
                ));
            }
        }
        let defect = req_bool(schedule, "defect_detected", at)?;
        let within_window = moisture.map(|m| m >= min && m <= max).unwrap_or(false);
        if evidence == "measured" && within_window && !defect {
            eligible.push((id, energy));
        }
    }
    eligible.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let selected = eligible
        .first()
        .ok_or_else(|| format!("{at}: no schedule passes the moisture and defect gates"))?;
    let expected = req_text(v, "expected_selected_schedule_id", at)?;
    if selected.0 != expected {
        return Err(format!(
            "{at}: expected selected schedule {expected}, independently recomputed {}",
            selected.0
        ));
    }
    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "metrics": {
            "schedule_count": schedules.len(),
            "eligible_schedule_count": eligible.len(),
            "selected_schedule_id": selected.0,
            "selected_synthetic_energy_units": selected.1
        }
    }))
}

fn validate_harvest_guard(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[3];
    exact_keys(
        v,
        &[
            "id",
            "evidence_class",
            "maximum_removal_units",
            "options",
            "expected_feasible_option_ids",
            "expected_assertions",
            "negative_controls",
        ],
        at,
    )?;
    note_arrays(v, at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong scenario identity or evidence class"));
    }
    let max_removal = req_u64(v, "maximum_removal_units", at)?;
    if max_removal == 0 {
        return Err(format!("{at}: maximum removal must be positive"));
    }
    let options = array(field(v, "options", at)?, &format!("{at}.options"))?;
    let mut ids = BTreeSet::new();
    let mut feasible = Vec::new();
    for option in options {
        exact_keys(
            option,
            &[
                "option_id",
                "planned_removal_units",
                "excluded_area_overlap_units",
                "regeneration_evidence_status",
                "ecological_gate_pass",
                "synthetic_net_value_units",
            ],
            &format!("{at}.option"),
        )?;
        let id = req_text(option, "option_id", at)?;
        if !ids.insert(id.clone()) {
            return Err(format!("{at}: duplicate option ID {id}"));
        }
        let removal = req_u64(option, "planned_removal_units", at)?;
        let overlap = req_u64(option, "excluded_area_overlap_units", at)?;
        if removal == 0 || overlap > removal {
            return Err(format!("{at}.{id}: invalid removal or exclusion overlap quantity"));
        }
        let evidence = req_text(option, "regeneration_evidence_status", at)?;
        if !["qualified", "estimated", "unknown"].contains(&evidence.as_str()) {
            return Err(format!("{at}.{id}: unsupported regeneration evidence state {evidence}"));
        }
        let ecological_pass = req_bool(option, "ecological_gate_pass", at)?;
        let _value = req_u64(option, "synthetic_net_value_units", at)?;
        if removal <= max_removal && overlap == 0 && evidence == "qualified" && ecological_pass {
            feasible.push(id);
        }
    }
    let expected = sorted_unique_strings(
        field(v, "expected_feasible_option_ids", at)?,
        &format!("{at}.expected_feasible_option_ids"),
    )?;
    require_same_ids(&feasible, &expected, at)?;
    let mut feasible_sorted = feasible.clone();
    feasible_sorted.sort();
    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "metrics": {
            "option_count": options.len(),
            "feasible_option_ids": feasible_sorted
        }
    }))
}

fn event_payload(event: &Value, at: &str) -> Check<Value> {
    let mut payload = object(event, at)?.clone();
    payload.remove("expected_result");
    Ok(Value::Object(payload))
}

fn validate_custody(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[4];
    exact_keys(
        v,
        &[
            "id",
            "evidence_class",
            "batch_id",
            "batch_quantity_units",
            "events",
            "expected_final_owner",
            "expected_rejected_transfers",
            "expected_idempotent_replays",
            "expected_appended_corrections",
            "expected_assertions",
            "negative_controls",
        ],
        at,
    )?;
    note_arrays(v, at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong scenario identity or evidence class"));
    }
    let batch_id = req_text(v, "batch_id", at)?;
    if batch_id.is_empty() {
        return Err(format!("{at}: batch ID must not be empty"));
    }
    let batch_quantity = req_u64(v, "batch_quantity_units", at)?;
    if batch_quantity == 0 {
        return Err(format!("{at}: batch quantity must be positive"));
    }
    let events = array(field(v, "events", at)?, &format!("{at}.events"))?;
    let mut seen: BTreeMap<String, Value> = BTreeMap::new();
    let mut batch_exists = false;
    let mut owner: Option<String> = None;
    let mut replays = 0_u64;
    let mut rejected_transfers = 0_u64;
    let mut corrections = 0_u64;

    for event in events {
        exact_keys(
            event,
            &[
                "event_id",
                "kind",
                "from_party",
                "to_party",
                "quantity_units",
                "correction_of",
                "expected_result",
            ],
            &format!("{at}.event"),
        )?;
        let id = req_text(event, "event_id", at)?;
        let kind = req_text(event, "kind", at)?;
        let from = req_text(event, "from_party", at)?;
        let to = req_text(event, "to_party", at)?;
        let quantity = req_u64(event, "quantity_units", at)?;
        let correction_of = req_text(event, "correction_of", at)?;
        let expected_result = req_text(event, "expected_result", at)?;
        if id.is_empty() {
            return Err(format!("{at}: event ID must not be empty"));
        }
        let payload = event_payload(event, at)?;
        if let Some(previous) = seen.get(&id) {
            if previous != &payload {
                return Err(format!("{at}: event ID {id} was replayed with a different payload"));
            }
            if expected_result != "idempotent_noop" {
                return Err(format!("{at}.{id}: identical event replay must be an idempotent no-op"));
            }
            replays = add(replays, 1, at)?;
            continue;
        }
        seen.insert(id.clone(), payload);

        let actual_result = match kind.as_str() {
            "create_batch" => {
                if batch_exists || from != "none" || to == "none" || to == from
                    || quantity != batch_quantity || correction_of != "none"
                {
                    return Err(format!("{at}.{id}: invalid batch creation event"));
                }
                batch_exists = true;
                owner = Some(to);
                "committed"
            }
            "transfer" => {
                if !batch_exists || from == "none" || to == "none" || from == to
                    || quantity != batch_quantity || correction_of != "none"
                {
                    return Err(format!("{at}.{id}: malformed transfer event"));
                }
                if owner.as_deref() == Some(from.as_str()) {
                    owner = Some(to);
                    "committed"
                } else {
                    rejected_transfers = add(rejected_transfers, 1, at)?;
                    "rejected_stale_owner"
                }
            }
            "correction" => {
                if from != "none" || to != "none" || quantity != 0
                    || correction_of == "none" || correction_of == id
                    || !seen.contains_key(&correction_of)
                {
                    return Err(format!("{at}.{id}: correction must link to an existing event and must not move custody"));
                }
                corrections = add(corrections, 1, at)?;
                "appended_correction"
            }
            _ => return Err(format!("{at}.{id}: unsupported event kind {kind}")),
        };
        if expected_result != actual_result {
            return Err(format!(
                "{at}.{id}: expected result {expected_result}, recomputed {actual_result}"
            ));
        }
    }

    let expected_owner = req_text(v, "expected_final_owner", at)?;
    if owner.as_deref() != Some(expected_owner.as_str()) {
        return Err(format!(
            "{at}: final owner mismatch; actual={owner:?}, expected={expected_owner}"
        ));
    }
    let expected_rejected = req_u64(v, "expected_rejected_transfers", at)?;
    let expected_replays = req_u64(v, "expected_idempotent_replays", at)?;
    let expected_corrections = req_u64(v, "expected_appended_corrections", at)?;
    if rejected_transfers != expected_rejected || replays != expected_replays || corrections != expected_corrections {
        return Err(format!(
            "{at}: count mismatch; rejected={rejected_transfers}/{expected_rejected}, replays={replays}/{expected_replays}, corrections={corrections}/{expected_corrections}"
        ));
    }
    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "metrics": {
            "event_count_including_replays": events.len(),
            "final_owner": owner,
            "rejected_transfers": rejected_transfers,
            "idempotent_replays": replays,
            "appended_corrections": corrections
        }
    }))
}

#[derive(Clone, Debug)]
struct ProductOption {
    id: String,
    cost: u64,
    energy: u64,
    impact: u64,
    output: u64,
    feasible: bool,
}

fn dominates(a: &ProductOption, b: &ProductOption) -> bool {
    let no_worse = a.cost <= b.cost
        && a.energy <= b.energy
        && a.impact <= b.impact
        && a.output >= b.output;
    let strictly_better = a.cost < b.cost
        || a.energy < b.energy
        || a.impact < b.impact
        || a.output > b.output;
    no_worse && strictly_better
}

fn validate_lifecycle_pareto(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[5];
    exact_keys(
        v,
        &[
            "id",
            "evidence_class",
            "minimum_qualified_output_units",
            "options",
            "expected_feasible_option_ids",
            "expected_pareto_option_ids",
            "expected_assertions",
            "negative_controls",
        ],
        at,
    )?;
    note_arrays(v, at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong scenario identity or evidence class"));
    }
    let required_output = req_u64(v, "minimum_qualified_output_units", at)?;
    if required_output == 0 {
        return Err(format!("{at}: qualified output floor must be positive"));
    }
    let options = array(field(v, "options", at)?, &format!("{at}.options"))?;
    if options.is_empty() {
        return Err(format!("{at}: options must not be empty"));
    }
    let mut ids = BTreeSet::new();
    let mut products: Vec<ProductOption> = Vec::new();
    for option in options {
        exact_keys(
            option,
            &[
                "option_id",
                "synthetic_cost_units",
                "synthetic_energy_units",
                "synthetic_impact_units",
                "qualified_output_units",
                "provenance_complete",
                "grade_evidence_status",
            ],
            &format!("{at}.option"),
        )?;
        let id = req_text(option, "option_id", at)?;
        if !ids.insert(id.clone()) {
            return Err(format!("{at}: duplicate option ID {id}"));
        }
        let cost = req_u64(option, "synthetic_cost_units", at)?;
        let energy = req_u64(option, "synthetic_energy_units", at)?;
        let impact = req_u64(option, "synthetic_impact_units", at)?;
        let output = req_u64(option, "qualified_output_units", at)?;
        let provenance = req_bool(option, "provenance_complete", at)?;
        let grade = req_text(option, "grade_evidence_status", at)?;
        if !["qualified", "missing", "unknown"].contains(&grade.as_str()) {
            return Err(format!("{at}.{id}: unsupported grade evidence state {grade}"));
        }
        let feasible = output >= required_output && provenance && grade == "qualified";
        products.push(ProductOption {
            id,
            cost,
            energy,
            impact,
            output,
            feasible,
        });
    }
    let mut feasible_ids: Vec<String> = products
        .iter()
        .filter(|p| p.feasible)
        .map(|p| p.id.clone())
        .collect();
    feasible_ids.sort();
    let expected_feasible = sorted_unique_strings(
        field(v, "expected_feasible_option_ids", at)?,
        &format!("{at}.expected_feasible_option_ids"),
    )?;
    require_same_ids(&feasible_ids, &expected_feasible, at)?;

    let feasible_products: Vec<&ProductOption> = products.iter().filter(|p| p.feasible).collect();
    let mut pareto_ids = Vec::new();
    for candidate in &feasible_products {
        let is_dominated = feasible_products
            .iter()
            .any(|other| other.id != candidate.id && dominates(other, candidate));
        if !is_dominated {
            pareto_ids.push(candidate.id.clone());
        }
    }
    pareto_ids.sort();
    let expected_pareto = sorted_unique_strings(
        field(v, "expected_pareto_option_ids", at)?,
        &format!("{at}.expected_pareto_option_ids"),
    )?;
    require_same_ids(&pareto_ids, &expected_pareto, at)?;

    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "metrics": {
            "option_count": options.len(),
            "feasible_option_ids": feasible_ids,
            "pareto_option_ids": pareto_ids
        }
    }))
}

fn validate_corpus(corpus: &Value) -> Check<Vec<Value>> {
    exact_keys(
        corpus,
        &[
            "suite_id",
            "suite_version",
            "manifest_kind",
            "created_date",
            "units",
            "claim_ceiling",
            "scenarios",
        ],
        "corpus",
    )?;
    if req_text(corpus, "suite_id", "corpus")? != "wood-products-lifecycle-reference"
        || req_text(corpus, "suite_version", "corpus")? != "0.1.0"
        || req_text(corpus, "manifest_kind", "corpus")? != "synthetic-contract-fixtures"
    {
        return Err("corpus: unsupported suite identity, version or manifest kind".to_owned());
    }
    let claim = req_text(corpus, "claim_ceiling", "corpus")?;
    if claim.is_empty() {
        return Err("corpus: claim ceiling must not be empty".to_owned());
    }
    exact_keys(
        field(corpus, "units", "corpus")?,
        &[
            "dry_mass",
            "moisture",
            "energy",
            "cost",
            "synthetic_impact",
            "qualified_output",
        ],
        "corpus.units",
    )?;
    let expected_units = [
        ("dry_mass", "gram"),
        ("moisture", "basis_point"),
        ("energy", "synthetic_energy_unit"),
        ("cost", "synthetic_cost_unit"),
        ("synthetic_impact", "synthetic_impact_unit"),
        ("qualified_output", "unit"),
    ];
    let units = field(corpus, "units", "corpus")?;
    for (key, expected) in expected_units {
        if text(field(units, key, "corpus.units")?, &format!("corpus.units.{key}"))? != expected {
            return Err(format!("corpus.units.{key}: unsupported unit"));
        }
    }

    let scenarios = array(field(corpus, "scenarios", "corpus")?, "corpus.scenarios")?;
    if scenarios.len() != REQUIRED_IDS.len() {
        return Err(format!(
            "corpus: expected {} scenarios, got {}",
            REQUIRED_IDS.len(),
            scenarios.len()
        ));
    }
    let mut unique = BTreeSet::new();
    let mut results = Vec::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        let id = req_text(scenario, "id", "scenario")?;
        if !unique.insert(id.clone()) {
            return Err(format!("corpus: duplicate scenario ID {id}"));
        }
        if id != REQUIRED_IDS[index] {
            return Err(format!(
                "corpus: scenario at index {index} must be {}, got {id}",
                REQUIRED_IDS[index]
            ));
        }
        let result = match id.as_str() {
            "SAWING_DRY_MASS_CLOSURE" => validate_mass_closure(scenario)?,
            "STRUCTURAL_GRADE_GATE" => validate_grade_gate(scenario)?,
            "DRYING_QUALITY_FLOOR" => validate_drying(scenario)?,
            "HARVEST_REGENERATION_GUARD" => validate_harvest_guard(scenario)?,
            "CUSTODY_REPLAY_AND_CORRECTION" => validate_custody(scenario)?,
            "LIFECYCLE_RECOVERY_PARETO" => validate_lifecycle_pareto(scenario)?,
            _ => return Err(format!("corpus: unsupported scenario ID {id}")),
        };
        results.push(result);
    }
    Ok(results)
}

fn valid_source_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn build_receipt(bytes: &[u8], source_commit: &str) -> Check<Value> {
    if !valid_source_commit(source_commit) {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }
    let corpus: Value = serde_json::from_slice(bytes)
        .map_err(|err| format!("invalid corpus JSON: {err}"))?;
    let scenarios = validate_corpus(&corpus)?;
    let corpus_sha256 = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(json!({
        "validator": "wood-products-bench",
        "validator_version": VALIDATOR_VERSION,
        "result": "pass",
        "scenario_count": scenarios.len(),
        "corpus_sha256": corpus_sha256,
        "source_commit": source_commit,
        "scenarios": scenarios
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        eprintln!(
            "usage: wood-products-bench <corpus.json> --source-commit <40-character-commit-sha>"
        );
        return ExitCode::from(2);
    }
    let bytes = match fs::read(&args[0]) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("cannot read corpus: {err}");
            return ExitCode::FAILURE;
        }
    };
    match build_receipt(&bytes, &args[2]) {
        Ok(receipt) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt).expect("JSON receipt serializes")
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("wood-products benchmark rejected: {err}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../../../docs/tech/wood-products-benchmark-corpus-v0.1.json");
    const SOURCE_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn parsed_corpus() -> Value {
        serde_json::from_str(CORPUS).expect("frozen corpus is valid JSON")
    }

    fn validate_modified(mut corpus: Value) -> Check<Value> {
        corpus["created_date"] = json!("2026-10-16");
        let bytes = serde_json::to_vec(&corpus).expect("mutated corpus serializes");
        build_receipt(&bytes, SOURCE_COMMIT)
    }

    #[test]
    fn frozen_corpus_produces_receipt_for_six_scenarios() {
        let receipt = build_receipt(CORPUS.as_bytes(), SOURCE_COMMIT).expect("corpus validates");
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["scenario_count"], 6);
        assert_eq!(
            receipt["scenarios"][0]["metrics"]["accounted_dry_mass_g"],
            1_000_000
        );
        assert_eq!(
            receipt["scenarios"][4]["metrics"]["final_owner"],
            "buyer_a"
        );
        assert_eq!(
            receipt["scenarios"][5]["metrics"]["pareto_option_ids"],
            json!(["reclaimed_certified", "virgin_certified"])
        );
    }

    #[test]
    fn mass_imbalance_is_rejected() {
        let mut corpus = parsed_corpus();
        corpus["scenarios"][0]["material_streams"][0]["dry_mass_g"] = json!(620_001);
        assert!(validate_modified(corpus).is_err());
    }

    #[test]
    fn missing_grade_evidence_cannot_win_on_margin() {
        let mut corpus = parsed_corpus();
        corpus["scenarios"][1]["options"][1]["grade_evidence_status"] = json!("missing");
        assert!(validate_modified(corpus).is_err());
    }

    #[test]
    fn duplicate_event_id_with_changed_payload_is_rejected() {
        let mut corpus = parsed_corpus();
        corpus["scenarios"][4]["events"][1]["to_party"] = json!("malicious_party");
        assert!(validate_modified(corpus).is_err());
    }

    #[test]
    fn unknown_root_key_is_rejected() {
        let mut corpus = parsed_corpus();
        corpus["unmodeled_default"] = json!(0);
        assert!(validate_modified(corpus).is_err());
    }

    #[test]
    fn a_dominated_option_cannot_be_added_to_the_pareto_frontier() {
        let mut corpus = parsed_corpus();
        corpus["scenarios"][5]["expected_pareto_option_ids"] =
            json!(["inefficient_virgin", "reclaimed_certified", "virgin_certified"]);
        assert!(validate_modified(corpus).is_err());
    }
}
