use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::process::ExitCode;

const VALIDATOR_VERSION: &str = "0.1.0";
const REQUIRED_CASES: [&str; 27] = [
    "BASELINE_EXISTING_PROVIDER",
    "COASTAL_GATEWAY_ADJACENT_RANGE",
    "SEPARATED_PORT_INTEGRATION_LAUNCH",
    "INLAND_CONVENTIONAL_RAIL_ORIGIN",
    "ROAD_ONLY_CAPACITY_SHORTFALL",
    "MAGRAIL_LINEAR_MOTOR_PILOT",
    "DEDICATED_UNDERGROUND_MAGLEV_CONCEPT",
    "CONVENTIONAL_RAIL_SHORT_TUNNEL_BYPASS",
    "EXISTING_PROVIDER_NO_BUILD_BASELINE",
    "PORT_CAPACITY_BOTTLENECK",
    "PAYLOAD_INTEGRATION_CAPACITY_BOTTLENECK",
    "LAUNCH_RANGE_CAPACITY_BOTTLENECK",
    "MISSED_LAUNCH_WINDOW_DEADLINE",
    "RANGE_CLOSURE",
    "POLAR_MISSION_NORTHBOUND",
    "LOW_INCLINATION_EQUATORWARD_ACCESS",
    "INVALID_EQUATORIAL_ORBIT_ASSUMPTION",
    "SHARED_GATEWAY_COMMON_MODE_N_MINUS_ONE",
    "INDEPENDENT_GATEWAYS_N_MINUS_ONE",
    "UNKNOWN_SHARED_GRID_CAPACITY",
    "SUPPLIER_DELAY_EXCEEDS_CAMPAIGN_DEADLINE",
    "PAYLOAD_MASS_LOSS_AT_TRANSFER",
    "FUTURE_ORBITAL_RING_CONSTRUCTION_CARGO",
    "LINEAR_MOTOR_MISLABELED_AS_MAGLEV",
    "UNQUALIFIED_RING_CLAIMED_OPERATIONAL",
    "SLANTED_GEO_TETHER_CONCEPT",
    "MUTUALLY_EXCLUSIVE_PATHS_NOT_ADDED_AS_CAPACITY",
];

type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct RouteResult {
    id: String,
    status: String,
    reason_codes: Vec<String>,
    guaranteed_capacity_kg: u64,
    potential_capacity_kg: u64,
    cost_scu: u64,
    energy_wh: u64,
    duration_minutes: u64,
    dependency_domains: BTreeSet<String>,
}

fn object<'a>(value: &'a Value, at: &str) -> Check<&'a Map<String, Value>> {
    value.as_object().ok_or_else(|| format!("{at}: expected object"))
}

fn exact_keys(value: &Value, keys: &[&str], at: &str) -> Check<()> {
    let actual = object(value, at)?;
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual_keys: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual_keys).copied().collect();
    let unknown: Vec<_> = actual_keys.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}: missing field {key}"))
}

fn text(value: &Value, at: &str) -> Check<String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}: expected string"))
}

fn integer(value: &Value, at: &str) -> Check<u64> {
    value
        .as_u64()
        .ok_or_else(|| format!("{at}: expected non-negative integer"))
}

fn boolean(value: &Value, at: &str) -> Check<bool> {
    value
        .as_bool()
        .ok_or_else(|| format!("{at}: expected boolean"))
}

fn array<'a>(value: &'a Value, at: &str) -> Check<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| format!("{at}: expected array"))
}

fn req_text(value: &Value, key: &str, at: &str) -> Check<String> {
    text(field(value, key, at)?, &format!("{at}.{key}"))
}

fn req_u64(value: &Value, key: &str, at: &str) -> Check<u64> {
    integer(field(value, key, at)?, &format!("{at}.{key}"))
}

fn checked_add(left: u64, right: u64, at: &str) -> Check<u64> {
    left.checked_add(right)
        .ok_or_else(|| format!("{at}: integer overflow"))
}

fn set_status(status: &mut String, candidate: &str) {
    let rank = |value: &str| match value {
        "feasible" => 0,
        "indeterminate" => 1,
        "infeasible" => 2,
        "invalid" => 3,
        _ => 4,
    };
    if rank(candidate) > rank(status) {
        *status = candidate.to_owned();
    }
}

fn gate_state(value: &Value, at: &str) -> Check<String> {
    let state = text(value, at)?;
    if !matches!(state.as_str(), "pass" | "fail" | "unknown") {
        return Err(format!("{at}: unsupported gate state {state}"));
    }
    Ok(state)
}

fn orbit_inclination_mdeg(latitude_mdeg: i64, azimuth_mdeg: u64) -> Check<f64> {
    if !(-90_000..=90_000).contains(&latitude_mdeg) || azimuth_mdeg > 360_000 {
        return Err("launch geometry latitude or azimuth is outside its physical range".to_owned());
    }
    let latitude = (latitude_mdeg as f64 / 1000.0).to_radians();
    let azimuth = (azimuth_mdeg as f64 / 1000.0).to_radians();
    let cos_inclination = latitude.cos() * azimuth.sin();
    Ok(cos_inclination.clamp(-1.0, 1.0).acos().to_degrees() * 1000.0)
}

fn evaluate_route(route: &Value, mission: &Value) -> Check<RouteResult> {
    let route_id = req_text(route, "id", "route")?;
    exact_keys(route, &["id", "stages"], &format!("route.{route_id}"))?;

    let payload_mass = req_u64(mission, "payload_mass_kg", "mission")?;
    let target_inclination = req_u64(mission, "target_inclination_mdeg", "mission")?;
    let deadline = req_u64(mission, "deadline_minutes", "mission")?;
    let stages = array(field(route, "stages", &route_id)?, &format!("{route_id}.stages"))?;
    if stages.is_empty() {
        return Err(format!("{route_id}: route requires at least one stage"));
    }

    let mut seen_stage_ids = BTreeSet::new();
    let mut dependencies = BTreeSet::new();
    let mut reason_codes = Vec::new();
    let mut status = "feasible".to_owned();
    let mut bottleneck = payload_mass;
    let mut potential_bottleneck = payload_mass;
    let mut has_unknown_capacity = false;
    let mut cost = 0_u64;
    let mut energy = 0_u64;
    let mut duration = 0_u64;

    for stage in stages {
        let stage_id = req_text(stage, "id", &route_id)?;
        let at = format!("{route_id}.{stage_id}");
        exact_keys(
            stage,
            &[
                "id",
                "technology_family",
                "capacity_state",
                "capacity_kg",
                "input_mass_kg",
                "output_mass_kg",
                "duration_minutes",
                "cost_scu",
                "energy_wh",
                "dependency_domains",
                "hard_gates",
                "levitation_claim",
                "capability_maturity",
                "operational_capability_claim",
                "launch_geometry",
            ],
            &at,
        )?;
        if !seen_stage_ids.insert(stage_id.clone()) {
            return Err(format!("{route_id}: duplicate stage id {stage_id}"));
        }

        let technology = req_text(stage, "technology_family", &at)?;
        let allowed_technologies = [
            "conventional_rail",
            "road_freight",
            "port_transfer",
            "payload_integration",
            "rocket_launch",
            "linear_motor_rail",
            "maglev_freight",
            "underground_pod",
            "conveyor",
            "pipeline",
            "orbital_ring",
            "skyhook",
            "space_elevator",
            "space_elevator_slanted",
            "launch_loop",
        ];
        if !allowed_technologies.contains(&technology.as_str()) {
            return Err(format!("{at}: unsupported technology family {technology}"));
        }

        let input_mass = req_u64(stage, "input_mass_kg", &at)?;
        let output_mass = req_u64(stage, "output_mass_kg", &at)?;
        if input_mass != payload_mass || output_mass != payload_mass {
            set_status(&mut status, "invalid");
            reason_codes.push("CARGO_MASS_NOT_CONSERVED".to_owned());
        }

        let capacity_state = req_text(stage, "capacity_state", &at)?;
        let stage_capacity = req_u64(stage, "capacity_kg", &at)?;
        match capacity_state.as_str() {
            "known" if stage_capacity > 0 => {
                bottleneck = bottleneck.min(stage_capacity);
                potential_bottleneck = potential_bottleneck.min(stage_capacity);
            }
            "unknown" if stage_capacity == 0 => {
                has_unknown_capacity = true;
                potential_bottleneck = potential_bottleneck.min(payload_mass);
                set_status(&mut status, "indeterminate");
                reason_codes.push("CAPACITY_UNKNOWN".to_owned());
            }
            "known" => return Err(format!("{at}: known capacity must be positive")),
            "unknown" => return Err(format!("{at}: unknown capacity must be encoded as zero")),
            _ => return Err(format!("{at}: unsupported capacity state {capacity_state}")),
        }

        let stage_duration = req_u64(stage, "duration_minutes", &at)?;
        if stage_duration == 0 {
            return Err(format!("{at}: duration must be positive"));
        }
        duration = checked_add(duration, stage_duration, &at)?;
        cost = checked_add(cost, req_u64(stage, "cost_scu", &at)?, &at)?;
        energy = checked_add(energy, req_u64(stage, "energy_wh", &at)?, &at)?;

        let domain_values = array(field(stage, "dependency_domains", &at)?, &format!("{at}.dependency_domains"))?;
        let mut stage_domains = BTreeSet::new();
        for domain_value in domain_values {
            let domain = text(domain_value, &format!("{at}.dependency_domains"))?;
            if domain.trim().is_empty() || !stage_domains.insert(domain.clone()) {
                return Err(format!("{at}: empty or duplicate dependency domain"));
            }
            dependencies.insert(domain);
        }

        let gates = field(stage, "hard_gates", &at)?;
        exact_keys(gates, &["rights", "safety", "environment"], &format!("{at}.hard_gates"))?;
        for gate_name in ["rights", "safety", "environment"] {
            match gate_state(field(gates, gate_name, &at)?, &format!("{at}.hard_gates.{gate_name}"))?.as_str() {
                "pass" => {}
                "fail" => {
                    set_status(&mut status, "infeasible");
                    reason_codes.push(format!("HARD_GATE_{gate_name.to_uppercase()}_FAILED"));
                }
                "unknown" => {
                    set_status(&mut status, "indeterminate");
                    reason_codes.push(format!("HARD_GATE_{gate_name.to_uppercase()}_UNKNOWN"));
                }
                _ => unreachable!(),
            }
        }

        let levitation_claim = boolean(field(stage, "levitation_claim", &at)?, &at)?;
        if (technology == "maglev_freight") != levitation_claim {
            set_status(&mut status, "invalid");
            reason_codes.push("MAGNETIC_LEVITATION_LABEL_MISMATCH".to_owned());
        }

        let maturity = req_text(stage, "capability_maturity", &at)?;
        if !matches!(
            maturity.as_str(),
            "operational_reference" | "pilot_reference" | "concept_only" | "unknown"
        ) {
            return Err(format!("{at}: unsupported capability maturity {maturity}"));
        }
        let operational_claim = boolean(
            field(stage, "operational_capability_claim", &at)?,
            &at,
        )?;
        if operational_claim && maturity != "operational_reference" {
            set_status(&mut status, "invalid");
            reason_codes.push("UNQUALIFIED_CAPABILITY_CLAIMED_OPERATIONAL".to_owned());
        } else if maturity != "operational_reference" {
            set_status(&mut status, "indeterminate");
            reason_codes.push("CAPABILITY_NOT_OPERATIONAL_REFERENCE".to_owned());
        }

        let geometry = field(stage, "launch_geometry", &at)?;
        if technology == "rocket_launch" {
            if geometry.is_null() {
                return Err(format!("{at}: rocket launch requires explicit launch geometry"));
            }
            exact_keys(
                geometry,
                &[
                    "site_latitude_mdeg",
                    "launch_azimuth_mdeg",
                    "target_inclination_mdeg",
                    "range_gate",
                    "overflight_gate",
                ],
                &format!("{at}.launch_geometry"),
            )?;
            let latitude = field(geometry, "site_latitude_mdeg", &at)?
                .as_i64()
                .ok_or_else(|| format!("{at}: latitude must be a signed integer"))?;
            let azimuth = req_u64(geometry, "launch_azimuth_mdeg", &at)?;
            let geometry_target = req_u64(geometry, "target_inclination_mdeg", &at)?;
            if geometry_target != target_inclination || geometry_target > 180_000 {
                set_status(&mut status, "infeasible");
                reason_codes.push("TARGET_ORBIT_GEOMETRY_MISMATCH".to_owned());
            } else {
                let actual = orbit_inclination_mdeg(latitude, azimuth)?;
                if (actual - geometry_target as f64).abs() > 100.0 {
                    set_status(&mut status, "infeasible");
                    reason_codes.push("LAUNCH_AZIMUTH_CANNOT_REACH_TARGET_INCLINATION".to_owned());
                }
            }
            for gate_name in ["range_gate", "overflight_gate"] {
                match gate_state(
                    field(geometry, gate_name, &at)?,
                    &format!("{at}.launch_geometry.{gate_name}"),
                )?
                .as_str()
                {
                    "pass" => {}
                    "fail" => {
                        status = "infeasible".to_owned();
                        reason_codes.push(format!("LAUNCH_{gate_name.to_uppercase()}_FAILED"));
                    }
                    "unknown" => {
                        set_status(&mut status, "indeterminate");
                        reason_codes.push(format!("LAUNCH_{gate_name.to_uppercase()}_UNKNOWN"));
                    }
                    _ => unreachable!(),
                }
            }
        } else if !geometry.is_null() {
            return Err(format!("{at}: non-launch stage must not declare launch geometry"));
        }
    }

    if duration > deadline && status != "invalid" {
        set_status(&mut status, "infeasible");
        reason_codes.push("CAMPAIGN_DEADLINE_EXCEEDED".to_owned());
    }

    let guaranteed_capacity = if status == "feasible" { bottleneck } else { 0 };
    let potential_capacity = if matches!(status.as_str(), "invalid" | "infeasible") {
        0
    } else if has_unknown_capacity {
        potential_bottleneck.min(payload_mass)
    } else {
        potential_bottleneck
    };

    Ok(RouteResult {
        id: route_id,
        status,
        reason_codes,
        guaranteed_capacity_kg: guaranteed_capacity,
        potential_capacity_kg: potential_capacity,
        cost_scu: cost,
        energy_wh: energy,
        duration_minutes: duration,
        dependency_domains: dependencies,
    })
}

fn evaluate_scenario(scenario: &Value) -> Check<Value> {
    let id = req_text(scenario, "id", "scenario")?;
    let at = format!("scenario.{id}");
    exact_keys(
        scenario,
        &["id", "expected_status", "route_semantics", "require_n_minus_one", "mission", "routes"],
        &at,
    )?;
    let expected = req_text(scenario, "expected_status", &at)?;
    if !matches!(expected.as_str(), "feasible" | "infeasible" | "indeterminate" | "invalid") {
        return Err(format!("{at}: unsupported expected status {expected}"));
    }

    let mission = field(scenario, "mission", &at)?;
    exact_keys(
        mission,
        &[
            "payload_mass_kg",
            "deadline_minutes",
            "orbit_class",
            "target_inclination_mdeg",
        ],
        &format!("{at}.mission"),
    )?;
    let demand = req_u64(mission, "payload_mass_kg", &at)?;
    let deadline = req_u64(mission, "deadline_minutes", &at)?;
    let orbit_class = req_text(mission, "orbit_class", &at)?;
    let inclination = req_u64(mission, "target_inclination_mdeg", &at)?;
    if demand == 0 || deadline == 0 || inclination > 180_000 || orbit_class.trim().is_empty() {
        return Err(format!("{at}: invalid mission envelope"));
    }

    let route_semantics = req_text(scenario, "route_semantics", &at)?;
    if !matches!(route_semantics.as_str(), "alternatives" | "parallel_service") {
        return Err(format!("{at}: unsupported route semantics {route_semantics}"));
    }
    let require_n_minus_one = boolean(
        field(scenario, "require_n_minus_one", &at)?,
        &at,
    )?;
    if require_n_minus_one && route_semantics != "parallel_service" {
        return Err(format!("{at}: N-1 requires parallel_service route semantics"));
    }
    let route_values = array(field(scenario, "routes", &at)?, &format!("{at}.routes"))?;
    if route_values.is_empty() {
        return Err(format!("{at}: at least one route is required"));
    }
    let mut route_ids = BTreeSet::new();
    let mut routes = Vec::new();
    for route in route_values {
        let result = evaluate_route(route, mission)?;
        if !route_ids.insert(result.id.clone()) {
            return Err(format!("{at}: duplicate route id {}", result.id));
        }
        routes.push(result);
    }

    let mut reasons = Vec::new();
    let service_routes: Vec<&RouteResult> = routes
        .iter()
        .filter(|route| route.status != "invalid" && route.status != "infeasible")
        .collect();
    let mut result_status = if service_routes.is_empty() {
        if routes.iter().all(|route| route.status == "invalid") {
            reasons.push("ALL_ROUTE_ALTERNATIVES_INVALID".to_owned());
            "invalid"
        } else {
            reasons.push("NO_FEASIBLE_ROUTE".to_owned());
            "infeasible"
        }
    } else if service_routes.iter().any(|route| route.status == "feasible") {
        "feasible"
    } else {
        "indeterminate"
    }.to_owned();

    let (known_nominal, possible_nominal) = match route_semantics.as_str() {
        "alternatives" => (
            service_routes.iter().map(|route| route.guaranteed_capacity_kg).max().unwrap_or(0),
            service_routes.iter().map(|route| route.potential_capacity_kg).max().unwrap_or(0),
        ),
        "parallel_service" => (
            service_routes.iter().try_fold(0_u64, |sum, route| {
                checked_add(sum, route.guaranteed_capacity_kg, &at)
            })?,
            service_routes.iter().try_fold(0_u64, |sum, route| {
                checked_add(sum, route.potential_capacity_kg, &at)
            })?,
        ),
        _ => unreachable!(),
    };

    if matches!(result_status.as_str(), "feasible" | "indeterminate")
        && known_nominal < demand
    {
        if possible_nominal >= demand {
            if result_status == "feasible" {
                result_status = "indeterminate".to_owned();
            }
            reasons.push("NOMINAL_CAPACITY_DEPENDS_ON_UNQUALIFIED_OR_UNKNOWN_ROUTE".to_owned());
        } else {
            result_status = "infeasible".to_owned();
            reasons.push("NOMINAL_CAPACITY_BELOW_DEMAND".to_owned());
        }
    }

    if routes.iter().any(|route| route.status == "invalid") && result_status != "invalid" {
        reasons.push("INVALID_ALTERNATIVE_EXCLUDED_FROM_PORTFOLIO".to_owned());
    }

    let mut n_minus_one = Vec::new();
    if require_n_minus_one && result_status != "invalid" {
        let domains: BTreeSet<String> = service_routes
            .iter()
            .flat_map(|route| route.dependency_domains.iter().cloned())
            .collect();
        if domains.is_empty() {
            return Err(format!("{at}: N-1 requires at least one declared failure domain"));
        }
        for domain in domains {
            let known_after = service_routes
                .iter()
                .filter(|route| !route.dependency_domains.contains(&domain))
                .try_fold(0_u64, |sum, route| {
                    checked_add(sum, route.guaranteed_capacity_kg, &at)
                })?;
            let possible_after = service_routes
                .iter()
                .filter(|route| !route.dependency_domains.contains(&domain))
                .try_fold(0_u64, |sum, route| {
                    checked_add(sum, route.potential_capacity_kg, &at)
                })?;
            let disposition = if known_after >= demand {
                "pass"
            } else if possible_after >= demand {
                "indeterminate"
            } else {
                "fail"
            };
            if disposition == "fail" {
                result_status = "infeasible".to_owned();
                reasons.push("N_MINUS_ONE_SERVICE_FLOOR_FAILED".to_owned());
            } else if disposition == "indeterminate" && result_status == "feasible" {
                result_status = "indeterminate".to_owned();
                reasons.push("N_MINUS_ONE_SERVICE_FLOOR_UNKNOWN".to_owned());
            }
            n_minus_one.push(json!({
                "failed_domain": domain,
                "guaranteed_remaining_capacity_kg": known_after,
                "potential_remaining_capacity_kg": possible_after,
                "required_payload_kg": demand,
                "disposition": disposition
            }));
        }
    }

    let expected_match = result_status == expected;
    if !expected_match {
        return Err(format!(
            "{at}: expected {expected}, independently evaluated {result_status}"
        ));
    }

    let route_json: Vec<Value> = routes
        .iter()
        .map(|route| {
            json!({
                "route_id": route.id,
                "status": route.status,
                "reason_codes": route.reason_codes,
                "guaranteed_capacity_kg": route.guaranteed_capacity_kg,
                "potential_capacity_kg": route.potential_capacity_kg,
                "cost_scu": route.cost_scu,
                "energy_wh": route.energy_wh,
                "duration_minutes": route.duration_minutes,
                "dependency_domains": route.dependency_domains
            })
        })
        .collect();

    Ok(json!({
        "scenario_id": id,
        "expected_status": expected,
        "route_semantics": route_semantics,
        "status": result_status,
        "status_matches_expected": true,
        "mission": {
            "payload_mass_kg": demand,
            "orbit_class": orbit_class,
            "target_inclination_mdeg": inclination,
            "deadline_minutes": deadline
        },
        "nominal_guaranteed_capacity_kg": known_nominal,
        "nominal_potential_capacity_kg": possible_nominal,
        "n_minus_one": n_minus_one,
        "routes": route_json,
        "reason_codes": reasons
    }))
}

fn validate(root: &Value, source_commit: &str, digest: &str) -> Check<Value> {
    exact_keys(
        root,
        &[
            "suite_id",
            "suite_version",
            "manifest_kind",
            "evidence_class",
            "claim_ceiling",
            "scenarios",
        ],
        "root",
    )?;
    if req_text(root, "suite_id", "root")? != "earth-orbit-chain"
        || req_text(root, "suite_version", "root")? != "0.1.0"
        || req_text(root, "manifest_kind", "root")? != "synthetic-end-to-orbit-campaigns"
        || req_text(root, "evidence_class", "root")? != "synthetic"
    {
        return Err("root: unsupported suite identity, version or evidence class".to_owned());
    }
    if req_text(root, "claim_ceiling", "root")?.len() < 50 {
        return Err("root: claim ceiling missing".to_owned());
    }

    let scenarios = array(field(root, "scenarios", "root")?, "root.scenarios")?;
    if scenarios.len() != REQUIRED_CASES.len() {
        return Err(format!(
            "root: expected {} scenarios, found {}",
            REQUIRED_CASES.len(),
            scenarios.len()
        ));
    }
    let mut found = BTreeMap::new();
    for scenario in scenarios {
        let id = req_text(scenario, "id", "scenario")?;
        if !REQUIRED_CASES.contains(&id.as_str()) {
            return Err(format!("root: unknown scenario {id}"));
        }
        if found.insert(id.clone(), scenario).is_some() {
            return Err(format!("root: duplicate scenario {id}"));
        }
    }

    let mut reports = Vec::new();
    for id in REQUIRED_CASES {
        let scenario = found
            .get(id)
            .ok_or_else(|| format!("root: missing required scenario {id}"))?;
        reports.push(evaluate_scenario(scenario)?);
    }

    Ok(json!({
        "receipt_schema_version": "0.1.0",
        "validator_version": VALIDATOR_VERSION,
        "suite_id": "earth-orbit-chain",
        "suite_version": "0.1.0",
        "source_commit": source_commit,
        "corpus_sha256": digest,
        "scenario_count": reports.len(),
        "result": "pass",
        "claim_ceiling": "The receipt validates synthetic campaign semantics only; it is not real-world site, transport, orbital, safety, licensing or economic qualification.",
        "scenarios": reports
    }))
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        return Err("usage: earth_orbit_chain_reference --source-commit <40-char-lowercase-git-sha>".to_owned());
    }
    if !valid_commit_sha(&args[2]) {
        return Err("source commit must be a 40-character lowercase SHA-1".to_owned());
    }

    let bytes = include_bytes!("../../../docs/tech/earth-orbit-chain-benchmark-corpus-v0.1.json");
    let digest = format!("{:x}", Sha256::digest(bytes));
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid campaign corpus JSON: {error}"))?;
    let receipt = validate(&root, &args[2], &digest)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("earth_orbit_chain_reference: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_slice(include_bytes!(
            "../../../docs/tech/earth-orbit-chain-benchmark-corpus-v0.1.json"
        ))
        .expect("frozen synthetic campaign corpus parses")
    }

    fn result_for(root: &Value, id: &str) -> Value {
        let scenario = root["scenarios"]
            .as_array()
            .expect("scenarios array")
            .iter()
            .find(|scenario| scenario["id"] == id)
            .expect("required scenario exists");
        evaluate_scenario(scenario).expect("scenario evaluates")
    }

    #[test]
    fn all_twenty_six_frozen_scenarios_match_the_independent_oracle() {
        let root = corpus();
        let bytes = include_bytes!(
            "../../../docs/tech/earth-orbit-chain-benchmark-corpus-v0.1.json"
        );
        let digest = format!("{:x}", Sha256::digest(bytes));
        let receipt = validate(&root, &"a".repeat(40), &digest)
            .expect("all frozen campaign dispositions match");
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["scenario_count"], 27);
        assert_eq!(
            receipt["scenarios"]
                .as_array()
                .expect("scenario reports")
                .iter()
                .filter(|scenario| scenario["status_matches_expected"] == true)
                .count(),
            26
        );
    }

    #[test]
    fn polar_launch_geometry_is_not_given_an_equatorial_bonus() {
        let root = corpus();
        let polar = result_for(&root, "POLAR_MISSION_NORTHBOUND");
        let mismatch = result_for(&root, "INVALID_EQUATORIAL_ORBIT_ASSUMPTION");
        assert_eq!(polar["status"], "feasible");
        assert_eq!(mismatch["status"], "infeasible");
        assert!((orbit_inclination_mdeg(5_000, 0).unwrap() - 90_000.0).abs() < 0.1);
    }

    #[test]
    fn shared_gateway_is_a_common_mode_failure_but_independent_paths_survive() {
        let root = corpus();
        let shared = result_for(&root, "SHARED_GATEWAY_COMMON_MODE_N_MINUS_ONE");
        let independent = result_for(&root, "INDEPENDENT_GATEWAYS_N_MINUS_ONE");
        assert_eq!(shared["status"], "infeasible");
        assert_eq!(independent["status"], "feasible");
        assert!(
            shared["n_minus_one"]
                .as_array()
                .unwrap()
                .iter()
                .any(|item| item["failed_domain"] == "gateway:shared"
                    && item["guaranteed_remaining_capacity_kg"] == 0)
        );
    }

    #[test]
    fn linear_motor_pilot_is_not_promoted_to_maglev_or_operational_service() {
        let root = corpus();
        assert_eq!(
            result_for(&root, "MAGRAIL_LINEAR_MOTOR_PILOT")["status"],
            "indeterminate"
        );
        assert_eq!(
            result_for(&root, "LINEAR_MOTOR_MISLABELED_AS_MAGLEV")["status"],
            "invalid"
        );
    }

    #[test]
    fn mass_loss_and_unqualified_orbital_claims_are_rejected() {
        let root = corpus();
        assert_eq!(
            result_for(&root, "PAYLOAD_MASS_LOSS_AT_TRANSFER")["status"],
            "invalid"
        );
        assert_eq!(
            result_for(&root, "UNQUALIFIED_RING_CLAIMED_OPERATIONAL")["status"],
            "invalid"
        );
    }

    #[test]
    fn one_invalid_route_does_not_poison_an_independent_valid_alternative() {
        let root = corpus();
        let mut scenario = root["scenarios"][0].clone();
        let invalid = root["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "LINEAR_MOTOR_MISLABELED_AS_MAGLEV")
            .unwrap()["routes"][0]
            .clone();
        scenario["routes"].as_array_mut().unwrap().push(invalid);
        scenario["id"] = json!("MIXED_VALID_AND_INVALID_ALTERNATIVES");
        scenario["expected_status"] = json!("feasible");
        let result = evaluate_scenario(&scenario).expect("valid alternative remains usable");
        assert_eq!(result["status"], "feasible");
        assert_eq!(result["routes"][1]["status"], "invalid");
        assert_eq!(
            result["reason_codes"][0],
            "INVALID_ALTERNATIVE_EXCLUDED_FROM_PORTFOLIO"
        );
    }

    #[test]
    fn mutually_exclusive_route_alternatives_are_not_summed_as_parallel_capacity() {
        let root = corpus();
        let result = result_for(&root, "MUTUALLY_EXCLUSIVE_PATHS_NOT_ADDED_AS_CAPACITY");
        assert_eq!(result["route_semantics"], "alternatives");
        assert_eq!(result["nominal_guaranteed_capacity_kg"], 600);
        assert_eq!(result["status"], "infeasible");
    }

    #[test]
    fn concept_level_ring_and_slanted_tether_remain_indeterminate() {
        let root = corpus();
        assert_eq!(
            result_for(&root, "FUTURE_ORBITAL_RING_CONSTRUCTION_CARGO")["status"],
            "indeterminate"
        );
        assert_eq!(
            result_for(&root, "SLANTED_GEO_TETHER_CONCEPT")["status"],
            "indeterminate"
        );
    }

    #[test]
    fn unknown_capacity_never_becomes_zero_cost_or_unlimited_service() {
        let root = corpus();
        let unknown = result_for(&root, "UNKNOWN_SHARED_GRID_CAPACITY");
        assert_eq!(unknown["status"], "indeterminate");
        assert_eq!(unknown["nominal_guaranteed_capacity_kg"], 0);
        assert_eq!(unknown["nominal_potential_capacity_kg"], 1000);
    }

    #[test]
    fn rejects_duplicate_scenario_ids_and_mass_mutation_against_expected_outcome() {
        let mut root = corpus();
        let scenarios = root["scenarios"].as_array_mut().unwrap();
        scenarios[1]["id"] = scenarios[0]["id"].clone();
        assert!(validate(&root, &"a".repeat(40), "digest").is_err());

        let mut root = corpus();
        root["scenarios"][0]["routes"][0]["stages"][0]["output_mass_kg"] = json!(999);
        assert!(evaluate_scenario(&root["scenarios"][0]).is_ok_and(|value| value["status"] == "invalid"));
        assert!(validate(&root, &"a".repeat(40), "digest").is_err());
    }
}
