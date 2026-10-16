use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VALIDATOR_VERSION: &str = "0.1.0";
const REQUIRED_IDS: [&str; 6] = [
    "LIFECYCLE_MASS_COST_TRADEOFF",
    "CISLUNAR_RELAY_N_MINUS_ONE",
    "DELAYED_MARKET_AND_DOUBLE_SALE",
    "PROCESS_MASS_BALANCE_AND_DUPLICATE_EVENT",
    "DEBRIS_GATE_BEFORE_COST",
    "ROBUST_COST_AND_SERVICE_FLOOR",
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
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
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
    v.as_u64().ok_or_else(|| format!("{at}: expected non-negative integer"))
}

fn bool_at(v: &Value, at: &str) -> Check<bool> {
    v.as_bool().ok_or_else(|| format!("{at}: expected boolean"))
}

fn array<'a>(v: &'a Value, at: &str) -> Check<&'a Vec<Value>> {
    v.as_array().ok_or_else(|| format!("{at}: expected array"))
}

fn req_text(v: &Value, key: &str, at: &str) -> Check<String> {
    text(field(v, key, at)?, &format!("{at}.{key}"))
}

fn req_u64(v: &Value, key: &str, at: &str) -> Check<u64> {
    u64_at(field(v, key, at)?, &format!("{at}.{key}"))
}

fn add(a: u64, b: u64, at: &str) -> Check<u64> {
    a.checked_add(b).ok_or_else(|| format!("{at}: integer overflow"))
}

fn mul(a: u64, b: u64, at: &str) -> Check<u64> {
    a.checked_mul(b).ok_or_else(|| format!("{at}: integer overflow"))
}

fn lifecycle(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[0];
    exact_keys(v, &["id", "evidence_class", "horizon_years", "discount_rate_basis_points", "required_service_units_total", "options", "expected_assertions", "negative_controls"], at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong identity or evidence class"));
    }
    let horizon = req_u64(v, "horizon_years", at)?;
    let required = req_u64(v, "required_service_units_total", at)?;
    if req_u64(v, "discount_rate_basis_points", at)? != 0 {
        return Err(format!("{at}: v0.1 fixture arithmetic requires a zero discount rate"));
    }
    if horizon == 0 || required == 0 {
        return Err(format!("{at}: horizon and required service must be positive"));
    }
    let options = array(field(v, "options", at)?, &format!("{at}.options"))?;
    if options.len() != 2 {
        return Err(format!("{at}: expected exactly two options"));
    }
    let mut by_id = BTreeMap::<String, (u64, u64)>::new();
    for opt in options {
        let id = req_text(opt, "id", at)?;
        let keys: &[&str] = if id == "local_isru" {
            &["id", "earth_launch_mass_kg", "capital_cost_scu", "annual_operating_cost_scu", "annual_maintenance_cost_scu", "service_units_per_year", "energy_kwh_per_service_unit"]
        } else {
            &["id", "earth_launch_mass_kg", "capital_cost_scu", "annual_operating_cost_scu", "annual_maintenance_cost_scu", "service_units_per_year"]
        };
        exact_keys(opt, keys, &format!("{at}.{id}"))?;
        if id != "earth_import" && id != "local_isru" {
            return Err(format!("{at}: unknown option {id}"));
        }
        let mass = req_u64(opt, "earth_launch_mass_kg", at)?;
        let capex = req_u64(opt, "capital_cost_scu", at)?;
        let annual_op = req_u64(opt, "annual_operating_cost_scu", at)?;
        let annual_maint = req_u64(opt, "annual_maintenance_cost_scu", at)?;
        let annual_service = req_u64(opt, "service_units_per_year", at)?;
        if mass == 0 || annual_service == 0 || mul(annual_service, horizon, at)? != required {
            return Err(format!("{at}.{id}: zero mass/service or service horizon mismatch"));
        }
        if id == "local_isru" {
            if req_u64(opt, "energy_kwh_per_service_unit", at)? == 0 {
                return Err(format!("{at}.{id}: energy intensity must be positive"));
            }
        }
        let annual = add(annual_op, annual_maint, at)?;
        let lifecycle_cost = add(capex, mul(annual, horizon, at)?, at)?;
        by_id.insert(id, (mass, lifecycle_cost));
    }
    let (import_mass, import_cost) = *by_id.get("earth_import").ok_or_else(|| format!("{at}: missing earth_import"))?;
    let (isru_mass, isru_cost) = *by_id.get("local_isru").ok_or_else(|| format!("{at}: missing local_isru"))?;
    if isru_mass >= import_mass || isru_cost <= import_cost {
        return Err(format!("{at}: fixture must show lower ISRU launch mass but higher lifecycle cost"));
    }
    let mass_reduction_percent = mul(import_mass - isru_mass, 100, at)? / import_mass;
    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "reason_codes": [],
        "metrics": {
            "earth_import_lifecycle_cost_scu": import_cost,
            "local_isru_lifecycle_cost_scu": isru_cost,
            "earth_import_mass_kg": import_mass,
            "local_isru_mass_kg": isru_mass,
            "earth_launched_mass_reduction_percent_integer": mass_reduction_percent,
            "required_service_units": required
        }
    }))
}

fn relay(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[1];
    exact_keys(v, &["id", "evidence_class", "service_window_id", "demand_capacity_units", "plans", "fault_cases", "expected_assertions", "negative_controls"], at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong identity or evidence class"));
    }
    let demand = req_u64(v, "demand_capacity_units", at)?;
    if demand == 0 { return Err(format!("{at}: demand must be positive")); }
    let plans = array(field(v, "plans", at)?, &format!("{at}.plans"))?;
    if plans.len() != 2 { return Err(format!("{at}: expected two plans")); }
    let mut capacities = BTreeMap::<String, BTreeMap<String, u64>>::new();
    let mut costs = BTreeMap::<String, u64>::new();
    for plan in plans {
        let id = req_text(plan, "id", at)?;
        exact_keys(plan, &["id", "providers"], &format!("{at}.{id}"))?;
        if id != "single_provider" && id != "dual_provider" { return Err(format!("{at}: unknown plan {id}")); }
        let providers = array(field(plan, "providers", at)?, &format!("{at}.{id}.providers"))?;
        if (id == "single_provider" && providers.len() != 1) || (id == "dual_provider" && providers.len() != 2) {
            return Err(format!("{at}.{id}: provider count mismatch"));
        }
        let mut caps = BTreeMap::new();
        let mut cost = 0;
        for provider in providers {
            exact_keys(provider, &["provider_id", "capacity_units", "cost_scu"], &format!("{at}.{id}.provider"))?;
            let pid = req_text(provider, "provider_id", at)?;
            let cap = req_u64(provider, "capacity_units", at)?;
            let c = req_u64(provider, "cost_scu", at)?;
            if cap == 0 || caps.insert(pid, cap).is_some() { return Err(format!("{at}.{id}: zero or duplicate provider")); }
            cost = add(cost, c, at)?;
        }
        capacities.insert(id.clone(), caps);
        costs.insert(id, cost);
    }
    let faults = array(field(v, "fault_cases", at)?, &format!("{at}.fault_cases"))?;
    if faults.len() != 3 { return Err(format!("{at}: expected exactly nominal and two single-provider fault cases")); }
    let mut seen_faults = BTreeSet::new();
    for fault in faults {
        exact_keys(fault, &["id", "failed_provider_ids"], &format!("{at}.fault"))?;
        let fid = req_text(fault, "id", at)?;
        if !seen_faults.insert(fid.clone()) { return Err(format!("{at}: duplicate fault case {fid}")); }
        let failed: BTreeSet<String> = array(field(fault, "failed_provider_ids", at)?, &format!("{at}.{fid}.failed_provider_ids"))?
            .iter().map(|x| text(x, &format!("{at}.{fid}.failed_provider_ids"))).collect::<Check<Vec<_>>>()?.into_iter().collect();
        let expected_failed: BTreeSet<String> = match fid.as_str() {
            "none" => BTreeSet::new(),
            "relay_a_down" => ["relay-a".to_owned()].into_iter().collect(),
            "relay_b_down" => ["relay-b".to_owned()].into_iter().collect(),
            _ => return Err(format!("{at}: unknown fault case {fid}")),
        };
        if failed != expected_failed { return Err(format!("{at}.{fid}: failed-provider set does not match the named fault")); }
        for plan_id in ["single_provider", "dual_provider"] {
            let available = capacities[plan_id].iter()
                .filter(|(pid, _)| !failed.contains(pid.as_str()))
                .try_fold(0_u64, |acc, (_, c)| add(acc, *c, at))?;
            if plan_id == "dual_provider" && available < demand {
                return Err(format!("{at}: dual-provider plan misses demand for {fid}"));
            }
            if plan_id == "single_provider" && fid == "none" && available < demand {
                return Err(format!("{at}: single-provider plan does not meet nominal demand"));
            }
            if plan_id == "single_provider" && fid == "relay_a_down" && available >= demand {
                return Err(format!("{at}: single-provider negative control unexpectedly survives relay-a loss"));
            }
        }
    }
    for required_fault in ["none", "relay_a_down", "relay_b_down"] {
        if !seen_faults.contains(required_fault) {
            return Err(format!("{at}: missing required fault case {required_fault}"));
        }
    }
    if costs.get("dual_provider") != Some(&65) {
        return Err(format!("{at}: expected dual-provider listed cost 65 SCU"));
    }
    Ok(json!({
        "scenario_id": at,
        "status": "pass",
        "reason_codes": [],
        "metrics": {
            "demand_capacity_units": demand,
            "dual_provider_total_cost_scu": costs["dual_provider"],
            "dual_provider_minimum_single_failure_capacity_units": capacities["dual_provider"].values().copied().min().unwrap_or(0)
        }
    }))
}

fn market(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[2];
    exact_keys(v, &["id", "evidence_class", "time_unit", "offer", "physical_asset", "sale_attempts", "replay", "expected_assertions", "negative_controls"], at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" || req_text(v, "time_unit", at)? != "logical_tick" {
        return Err(format!("{at}: wrong identity, evidence class or time unit"));
    }
    let offer = field(v, "offer", at)?;
    exact_keys(offer, &["offer_id", "created_tick", "expires_at_tick", "acceptance_tick"], &format!("{at}.offer"))?;
    let created = req_u64(offer, "created_tick", at)?;
    let expiry = req_u64(offer, "expires_at_tick", at)?;
    let accept = req_u64(offer, "acceptance_tick", at)?;
    if expiry < created || accept <= expiry { return Err(format!("{at}: offer must expire before the tested acceptance")); }
    let asset = field(v, "physical_asset", at)?;
    exact_keys(asset, &["asset_id", "quantity", "unit"], &format!("{at}.physical_asset"))?;
    if req_u64(asset, "quantity", at)? != 1 || req_text(asset, "unit", at)? != "asset" {
        return Err(format!("{at}: double-sale fixture must have exactly one physical asset"));
    }
    let sales = array(field(v, "sale_attempts", at)?, &format!("{at}.sale_attempts"))?;
    if sales.len() < 2 { return Err(format!("{at}: need competing sale attempts")); }
    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for sale in sales {
        exact_keys(sale, &["transaction_id", "buyer_id", "idempotency_key", "acceptance_tick"], &format!("{at}.sale_attempt"))?;
        if !ids.insert(req_text(sale, "transaction_id", at)?) || !keys.insert(req_text(sale, "idempotency_key", at)?) {
            return Err(format!("{at}: sale attempts must use distinct transaction and idempotency identifiers"));
        }
    }
    let replay = field(v, "replay", at)?;
    exact_keys(replay, &["transaction_id", "idempotency_key", "replay_count"], &format!("{at}.replay"))?;
    let replay_count = req_u64(replay, "replay_count", at)?;
    let replay_tx = req_text(replay, "transaction_id", at)?;
    let replay_key = req_text(replay, "idempotency_key", at)?;
    if replay_count < 2 || !sales.iter().any(|s| req_text(s, "transaction_id", at).ok().as_deref() == Some(replay_tx.as_str()) && req_text(s, "idempotency_key", at).ok().as_deref() == Some(replay_key.as_str())) {
        return Err(format!("{at}: replay must match a known sale and repeat at least once"));
    }
    Ok(json!({"scenario_id": at, "status": "pass", "reason_codes": [], "metrics": {
        "offer_expiry_tick": expiry, "tested_acceptance_tick": accept,
        "physical_asset_quantity": 1, "competing_sale_attempts": sales.len(), "replay_count": replay_count
    }}))
}

fn mass_balance(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[3];
    exact_keys(v, &["id", "evidence_class", "inventory", "process", "duplicate_event", "expected_assertions", "negative_controls"], at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong identity or evidence class"));
    }
    let inventory = field(v, "inventory", at)?;
    exact_keys(inventory, &["initial_measured_mass_kg", "import_event", "expected_total_mass_kg"], &format!("{at}.inventory"))?;
    let initial = req_u64(inventory, "initial_measured_mass_kg", at)?;
    let expected = req_u64(inventory, "expected_total_mass_kg", at)?;
    let import = field(inventory, "import_event", at)?;
    exact_keys(import, &["event_id", "verified_mass_kg"], &format!("{at}.inventory.import_event"))?;
    let event_id = req_text(import, "event_id", at)?;
    let imported = req_u64(import, "verified_mass_kg", at)?;
    if add(initial, imported, at)? != expected { return Err(format!("{at}: inventory total violates conservation")); }
    let duplicate = field(v, "duplicate_event", at)?;
    exact_keys(duplicate, &["event_id", "verified_mass_kg"], &format!("{at}.duplicate_event"))?;
    if req_text(duplicate, "event_id", at)? != event_id || req_u64(duplicate, "verified_mass_kg", at)? != imported {
        return Err(format!("{at}: duplicate event control must exactly repeat the import"));
    }
    let process = field(v, "process", at)?;
    exact_keys(process, &["process_id", "input_mass_kg", "outputs", "waste_mass_kg", "malformed_negative_control"], &format!("{at}.process"))?;
    let input = req_u64(process, "input_mass_kg", at)?;
    let output_sum = |streams: &Value| -> Check<u64> {
        array(streams, &format!("{at}.process.outputs"))?.iter().try_fold(0_u64, |acc, stream| {
            exact_keys(stream, &["stream_id", "mass_kg"], &format!("{at}.process.stream"))?;
            add(acc, req_u64(stream, "mass_kg", at)?, at)
        })
    };
    let valid_outputs = output_sum(field(process, "outputs", at)?)?;
    let waste = req_u64(process, "waste_mass_kg", at)?;
    if add(valid_outputs, waste, at)? != input { return Err(format!("{at}: nominal process violates mass balance")); }
    let malformed = field(process, "malformed_negative_control", at)?;
    exact_keys(malformed, &["outputs", "waste_mass_kg"], &format!("{at}.process.malformed_negative_control"))?;
    let malformed_outputs = output_sum(field(malformed, "outputs", at)?)?;
    let malformed_waste = req_u64(malformed, "waste_mass_kg", at)?;
    let malformed_total = add(malformed_outputs, malformed_waste, at)?;
    if malformed_total == input { return Err(format!("{at}: malformed negative control is not imbalanced")); }
    Ok(json!({"scenario_id": at, "status": "pass", "reason_codes": [], "metrics": {
        "inventory_mass_kg": expected, "process_input_mass_kg": input,
        "valid_output_plus_waste_mass_kg": add(valid_outputs, waste, at)?,
        "malformed_output_plus_waste_mass_kg": malformed_total,
        "malformed_imbalance_kg": malformed_total as i64 - input as i64
    }}))
}

fn debris_gate(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[4];
    exact_keys(v, &["id", "evidence_class", "orbital_regime", "applicability_note", "mission_specific_disposal_gate", "options", "expected_assertions", "negative_controls"], at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic"
        || req_text(v, "orbital_regime", at)? != "Earth_LEO"
        || req_text(v, "mission_specific_disposal_gate", at)? != "required" {
        return Err(format!("{at}: incorrect fixture identity or gate"));
    }
    if req_text(v, "applicability_note", at)?.len() < 20 {
        return Err(format!("{at}: mission-specific applicability boundary is missing"));
    }
    let options = array(field(v, "options", at)?, &format!("{at}.options"))?;
    if options.len() != 2 { return Err(format!("{at}: expected exactly two options")); }
    let mut found_low = false;
    let mut found_high = false;
    let mut low_cost = None;
    let mut high_cost = None;
    for option in options {
        exact_keys(option, &["id", "lifecycle_cost_scu", "disposal_gate_pass"], &format!("{at}.option"))?;
        let id = req_text(option, "id", at)?;
        let cost = req_u64(option, "lifecycle_cost_scu", at)?;
        let passes = bool_at(field(option, "disposal_gate_pass", at)?, at)?;
        match id.as_str() {
            "low_cost_noncompliant" => {
                found_low = true;
                low_cost = Some(cost);
                if passes { return Err(format!("{at}: low-cost negative control incorrectly passes its gate")); }
            }
            "higher_cost_compliant" => {
                found_high = true;
                high_cost = Some(cost);
                if !passes { return Err(format!("{at}: compliant option unexpectedly fails gate")); }
            }
            _ => return Err(format!("{at}: unknown option {id}")),
        }
    }
    if !found_low || !found_high { return Err(format!("{at}: required options missing")); }
    if high_cost <= low_cost {
        return Err(format!("{at}: compliant option must cost more than the noncompliant control in this fixture"));
    }
    Ok(json!({"scenario_id": at, "status": "pass", "reason_codes": [], "metrics": {
        "low_cost_option_rejected_by_gate": true, "only_compliant_option_feasible": true,
        "noncompliant_lifecycle_cost_scu": low_cost, "compliant_lifecycle_cost_scu": high_cost
    }}))
}

fn robust_service(v: &Value) -> Check<Value> {
    let at = REQUIRED_IDS[5];
    exact_keys(v, &["id", "evidence_class", "required_min_service_coverage_ppm", "stress_scenarios", "options", "expected_assertions", "negative_controls"], at)?;
    if req_text(v, "id", at)? != at || req_text(v, "evidence_class", at)? != "synthetic" {
        return Err(format!("{at}: wrong identity or evidence class"));
    }
    let floor = req_u64(v, "required_min_service_coverage_ppm", at)?;
    if floor > 1_000_000 { return Err(format!("{at}: ppm service floor exceeds 1,000,000")); }
    let stress = array(field(v, "stress_scenarios", at)?, &format!("{at}.stress_scenarios"))?;
    let stress_ids: Vec<String> = stress.iter().map(|x| text(x, at)).collect::<Check<Vec<_>>>()?;
    let required_stress = ["nominal", "communications_outage", "launch_slip"];
    if stress_ids.len() != 3 || required_stress.iter().any(|s| !stress_ids.iter().any(|x| x == s)) {
        return Err(format!("{at}: required stress scenarios missing"));
    }
    let options = array(field(v, "options", at)?, &format!("{at}.options"))?;
    if options.len() != 2 { return Err(format!("{at}: expected two options")); }
    let mut min_coverage = BTreeMap::<String, u64>::new();
    for option in options {
        exact_keys(option, &["id", "cost_by_scenario_scu", "service_coverage_ppm_by_scenario"], &format!("{at}.option"))?;
        let id = req_text(option, "id", at)?;
        if id != "fragile_low_bid" && id != "robust_provider" { return Err(format!("{at}: unknown option {id}")); }
        let costs = field(option, "cost_by_scenario_scu", at)?;
        let coverage = field(option, "service_coverage_ppm_by_scenario", at)?;
        exact_keys(costs, &required_stress, &format!("{at}.{id}.costs"))?;
        exact_keys(coverage, &required_stress, &format!("{at}.{id}.coverage"))?;
        let mut min_cov = u64::MAX;
        for scenario in required_stress {
            let c = u64_at(field(coverage, scenario, at)?, at)?;
            let _cost = u64_at(field(costs, scenario, at)?, at)?;
            if c > 1_000_000 { return Err(format!("{at}.{id}: invalid ppm value")); }
            min_cov = min_cov.min(c);
        }
        min_coverage.insert(id, min_cov);
    }
    if min_coverage["fragile_low_bid"] >= floor || min_coverage["robust_provider"] < floor {
        return Err(format!("{at}: stress fixtures do not separate fragile and robust options at the specified service floor"));
    }
    Ok(json!({"scenario_id": at, "status": "pass", "reason_codes": [], "metrics": {
        "required_min_service_coverage_ppm": floor,
        "fragile_low_bid_worst_coverage_ppm": min_coverage["fragile_low_bid"],
        "robust_provider_worst_coverage_ppm": min_coverage["robust_provider"]
    }}))
}

fn validate_annotations(v: &Value, at: &str) -> Check<()> {
    for key in ["expected_assertions", "negative_controls"] {
        let values = array(field(v, key, at)?, &format!("{at}.{key}"))?;
        if values.is_empty() {
            return Err(format!("{at}.{key}: must not be empty"));
        }
        for value in values {
            let sentence = text(value, &format!("{at}.{key}"))?;
            if sentence.trim().len() < 10 {
                return Err(format!("{at}.{key}: annotation too short to be actionable"));
            }
        }
    }
    Ok(())
}

fn validate(root: &Value, source_commit: &str, corpus_digest: &str) -> Check<Value> {
    exact_keys(root, &["suite_id", "suite_version", "manifest_kind", "created_date", "units", "claim_ceiling", "scenarios"], "root")?;
    if req_text(root, "suite_id", "root")? != "space-economy-reference"
        || req_text(root, "suite_version", "root")? != "0.1.0"
        || req_text(root, "manifest_kind", "root")? != "synthetic-contract-fixtures" {
        return Err("root: unsupported suite identity/version/kind".to_owned());
    }
    let units = field(root, "units", "root")?;
    exact_keys(units, &["cost", "mass", "energy", "service", "availability"], "units")?;
    if req_text(units, "cost", "units")? != "synthetic_cost_unit"
        || req_text(units, "mass", "units")? != "kg"
        || req_text(units, "energy", "units")? != "kWh"
        || req_text(units, "service", "units")? != "normalized_service_unit"
        || req_text(units, "availability", "units")? != "parts_per_million" {
        return Err("units: unexpected unit profile".to_owned());
    }
    if req_text(root, "claim_ceiling", "root")?.len() < 30 { return Err("root: claim ceiling missing".to_owned()); }
    let scenarios = array(field(root, "scenarios", "root")?, "scenarios")?;
    if scenarios.len() != REQUIRED_IDS.len() { return Err(format!("scenarios: expected {}, found {}", REQUIRED_IDS.len(), scenarios.len())); }
    let mut found = BTreeMap::<String, Value>::new();
    for scenario in scenarios {
        let id = req_text(scenario, "id", "scenario")?;
        if !REQUIRED_IDS.contains(&id.as_str()) { return Err(format!("scenario: unknown id {id}")); }
        validate_annotations(scenario, &id)?;
        if found.insert(id.clone(), scenario.clone()).is_some() { return Err(format!("scenario: duplicate id {id}")); }
    }
    let mut reports = Vec::new();
    for id in REQUIRED_IDS {
        let result = match id {
            "LIFECYCLE_MASS_COST_TRADEOFF" => lifecycle(&found[id]),
            "CISLUNAR_RELAY_N_MINUS_ONE" => relay(&found[id]),
            "DELAYED_MARKET_AND_DOUBLE_SALE" => market(&found[id]),
            "PROCESS_MASS_BALANCE_AND_DUPLICATE_EVENT" => mass_balance(&found[id]),
            "DEBRIS_GATE_BEFORE_COST" => debris_gate(&found[id]),
            "ROBUST_COST_AND_SERVICE_FLOOR" => robust_service(&found[id]),
            _ => unreachable!(),
        };
        reports.push(result?);
    }
    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "validator_version": VALIDATOR_VERSION,
        "suite_id": "space-economy-reference",
        "suite_version": "0.1.0",
        "source_commit": source_commit,
        "corpus_sha256": corpus_digest,
        "scenario_count": reports.len(),
        "result": "pass",
        "claim_ceiling": "Synthetic fixture integrity only; no real-world economics, physics, safety, or standards compliance is established.",
        "scenarios": reports
    }))
}

fn is_commit_sha(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err("usage: space-economy-bench <corpus.json> --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    let source_commit = &args[3];
    if !is_commit_sha(source_commit) { return Err("source commit must be a 40-character lowercase SHA-1".to_owned()); }
    let bytes = fs::read(&args[1]).map_err(|e| format!("cannot read corpus: {e}"))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let parsed: Value = serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON: {e}"))?;
    let receipt = validate(&parsed, source_commit, &digest)?;
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("space-economy-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_str(include_str!("../../../docs/tech/space-economy-benchmark-corpus-v0.1.json"))
            .expect("checked-in corpus must parse")
    }

    #[test]
    fn canonical_corpus_satisfies_all_six_synthetic_contracts() {
        let root = corpus();
        let digest = format!("{:x}", Sha256::digest(include_bytes!("../../../docs/tech/space-economy-benchmark-corpus-v0.1.json")));
        let receipt = validate(&root, &"a".repeat(40), &digest).expect("canonical corpus should validate");
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["scenario_count"], 6);
        assert_eq!(receipt["scenarios"][0]["metrics"]["earth_import_lifecycle_cost_scu"], 160);
        assert_eq!(receipt["scenarios"][0]["metrics"]["local_isru_lifecycle_cost_scu"], 375);
        assert_eq!(receipt["scenarios"][3]["metrics"]["malformed_imbalance_kg"], -1);
    }

    #[test]
    fn rejects_unknown_root_keys() {
        let mut root = corpus();
        root.as_object_mut().unwrap().insert("surprise".into(), json!(true));
        assert!(validate(&root, &"a".repeat(40), "digest").is_err());
    }

    #[test]
    fn rejects_duplicate_scenario_ids() {
        let mut root = corpus();
        let scenarios = root["scenarios"].as_array_mut().unwrap();
        scenarios[1]["id"] = scenarios[0]["id"].clone();
        assert!(validate(&root, &"a".repeat(40), "digest").is_err());
    }

    #[test]
    fn rejects_one_unit_mass_creation() {
        let mut root = corpus();
        root["scenarios"][3]["process"]["malformed_negative_control"]["outputs"][0]["mass_kg"] = json!(210);
        assert!(mass_balance(&root["scenarios"][3]).is_err());
    }

    #[test]
    fn rejects_robust_plan_that_falls_below_service_floor() {
        let mut root = corpus();
        root["scenarios"][5]["options"][1]["service_coverage_ppm_by_scenario"]["launch_slip"] = json!(969999);
        assert!(robust_service(&root["scenarios"][5]).is_err());
    }

    #[test]
    fn rejects_a_compliant_option_that_is_not_more_expensive_in_fixture() {
        let mut root = corpus();
        root["scenarios"][4]["options"][1]["lifecycle_cost_scu"] = json!(70);
        assert!(debris_gate(&root["scenarios"][4]).is_err());
    }
}
