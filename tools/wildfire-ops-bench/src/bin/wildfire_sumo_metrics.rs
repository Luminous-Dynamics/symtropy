use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const VERSION: &str = "0.1.0";
const SUMO_VERSION: &str = "1.27.1";
const CASES: [&str; 6] = [
    "TWO_ORIGINS_SHARED_BOTTLENECK_001",
    "FINITE_STORAGE_BACKPRESSURE_002",
    "ENTRY_CLOSURE_DURING_DEPARTURES_003",
    "ROUND_ROBIN_MERGE_FAIRNESS_004",
    "TOTAL_SHELTER_CAPACITY_HARD_GATE_005",
    "ACCESSIBLE_CAPACITY_HARD_GATE_006",
];
type Check<T> = Result<T, String>;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path) -> Check<Vec<u8>> {
    fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn parse_json(bytes: &[u8], label: &str) -> Check<Value> {
    serde_json::from_slice(bytes).map_err(|error| format!("parse {label}: {error}"))
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn array<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{at}.{key}: expected array"))
}

fn uint(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{at}.{key}: expected unsigned integer"))
}

fn valid_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

// Narrow parser for SUMO tripinfo start tags, not a general XML parser.
fn parse_attributes(body: &str) -> Check<BTreeMap<String, String>> {
    let bytes = body.as_bytes();
    let mut index = 0usize;
    let mut output = BTreeMap::new();

    while index < bytes.len() {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == bytes.len() || bytes[index] == b'/' {
            break;
        }

        let key_start = index;
        while index < bytes.len()
            && !bytes[index].is_ascii_whitespace()
            && bytes[index] != b'='
            && bytes[index] != b'/'
        {
            index += 1;
        }
        if key_start == index {
            return Err("empty XML attribute".to_owned());
        }
        let key = std::str::from_utf8(&bytes[key_start..index])
            .map_err(|_| "invalid attribute-name UTF-8".to_owned())?
            .to_owned();

        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == bytes.len() || bytes[index] != b'=' {
            return Err(format!("{key}: missing '='"));
        }
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index == bytes.len() || (bytes[index] != b'"' && bytes[index] != b'\'') {
            return Err(format!("{key}: unquoted value"));
        }

        let quote = bytes[index];
        index += 1;
        let value_start = index;
        while index < bytes.len() && bytes[index] != quote {
            index += 1;
        }
        if index == bytes.len() {
            return Err(format!("{key}: unterminated value"));
        }
        let value = std::str::from_utf8(&bytes[value_start..index])
            .map_err(|_| format!("{key}: invalid value UTF-8"))?
            .to_owned();
        index += 1;
        if output.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate attribute {key}"));
        }
    }

    Ok(output)
}

fn tripinfo_records(xml: &str) -> Check<Vec<BTreeMap<String, String>>> {
    if !xml.starts_with("<?xml") || !xml.contains("</tripinfos>") {
        return Err("incomplete tripinfo XML".to_owned());
    }
    let mut records = Vec::new();
    for part in xml.split("<tripinfo ").skip(1) {
        let (body, _) = part
            .split_once('>')
            .ok_or_else(|| "unterminated tripinfo tag".to_owned())?;
        records.push(parse_attributes(body)?);
    }
    Ok(records)
}

fn number(attributes: &BTreeMap<String, String>, key: &str) -> Check<Option<f64>> {
    let Some(raw) = attributes.get(key) else {
        return Ok(None);
    };
    let value = raw
        .parse::<f64>()
        .map_err(|_| format!("tripinfo.{key}: invalid number {raw}"))?;
    if !value.is_finite() {
        return Err(format!("tripinfo.{key}: non-finite number"));
    }
    Ok(Some(value))
}

fn required_number(attributes: &BTreeMap<String, String>, key: &str) -> Check<f64> {
    number(attributes, key)?.ok_or_else(|| format!("tripinfo missing {key}"))
}

fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((p * sorted.len() as f64).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1);
    Some(sorted[index])
}

fn distribution(values: &[f64]) -> Value {
    json!({
        "count": values.len(),
        "mean": if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f64>() / values.len() as f64)
        },
        "p50_nearest_rank": percentile(values, 0.50),
        "p90_nearest_rank": percentile(values, 0.90),
        "p95_nearest_rank": percentile(values, 0.95),
        "max": values.iter().copied().max_by(f64::total_cmp)
    })
}

fn expected_vehicle_ids(case: &Value, case_id: &str) -> Check<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for demand in array(case, "demands", case_id)? {
        let demand_id = field(demand, "id", case_id)?;
        let count = uint(demand, "vehicles", demand_id)?;
        if count > 50_000 {
            return Err(format!("{case_id}: demand count exceeds guardrail"));
        }
        for index in 0..count {
            let vehicle_id = format!("{demand_id}_{index:05}");
            if !ids.insert(vehicle_id.clone()) {
                return Err(format!("{case_id}: duplicate fixture vehicle {vehicle_id}"));
            }
        }
    }
    if ids.is_empty() {
        return Err(format!("{case_id}: empty vehicle inventory"));
    }
    Ok(ids)
}

fn measure_case(case: &Value, case_id: &str, xml: &str, horizon: u64) -> Check<Value> {
    let expected = expected_vehicle_ids(case, case_id)?;
    let records = tripinfo_records(xml)?;
    let mut seen = BTreeSet::new();
    let mut departed = 0usize;
    let mut undeparted = 0usize;
    let mut arrived = 0usize;
    let mut reroute_observations = 0usize;
    let mut rerouted = 0usize;
    let mut departures = Vec::new();
    let mut arrivals = Vec::new();
    let mut durations = Vec::new();
    let mut route_lengths = Vec::new();
    let mut waiting_times = Vec::new();
    let mut time_losses = Vec::new();
    let mut departure_delays = Vec::new();

    for attributes in &records {
        let vehicle_id = attributes
            .get("id")
            .ok_or_else(|| format!("{case_id}: tripinfo record missing id"))?;
        if !expected.contains(vehicle_id) {
            return Err(format!("{case_id}: unexpected vehicle {vehicle_id}"));
        }
        if !seen.insert(vehicle_id.clone()) {
            return Err(format!("{case_id}: duplicate vehicle {vehicle_id}"));
        }

        let depart = number(attributes, "depart")?;
        let arrival = number(attributes, "arrival")?;
        if depart.is_some_and(|value| value >= 0.0) {
            departed += 1;
            if let Some(delay) = number(attributes, "departDelay")? {
                if delay >= 0.0 {
                    departure_delays.push(delay);
                }
            }
        } else {
            undeparted += 1;
        }

        if let Some(raw) = attributes.get("rerouteNo") {
            let count = raw
                .parse::<u64>()
                .map_err(|_| format!("{case_id}: invalid rerouteNo"))?;
            reroute_observations += 1;
            if count > 0 {
                rerouted += 1;
            }
        }

        if arrival.is_some_and(|value| value >= 0.0) {
            let depart = depart
                .filter(|value| *value >= 0.0)
                .ok_or_else(|| format!("{case_id}: arrived trip missing departure"))?;
            let arrival = arrival.expect("nonnegative arrival checked above");
            let duration = required_number(attributes, "duration")?;
            let route_length = required_number(attributes, "routeLength")?;
            let waiting = required_number(attributes, "waitingTime")?;
            let loss = required_number(attributes, "timeLoss")?;
            if arrival < depart
                || [duration, route_length, waiting, loss]
                    .iter()
                    .any(|value| *value < 0.0)
            {
                return Err(format!(
                    "{case_id}: invalid completed-trip metrics for {vehicle_id}"
                ));
            }
            arrived += 1;
            departures.push(depart);
            arrivals.push(arrival);
            durations.push(duration);
            route_lengths.push(route_length);
            waiting_times.push(waiting);
            time_losses.push(loss);
        }
    }

    if seen != expected {
        let missing: Vec<_> = expected.difference(&seen).take(5).collect();
        return Err(format!(
            "{case_id}: expected {} IDs, found {}; missing examples {missing:?}",
            expected.len(),
            seen.len()
        ));
    }
    if records.len() != expected.len() {
        return Err(format!("{case_id}: tripinfo record inventory mismatch"));
    }

    let first_departure = departures.iter().copied().min_by(f64::total_cmp);
    let last_arrival = arrivals.iter().copied().max_by(f64::total_cmp);
    let observed_span = first_departure
        .zip(last_arrival)
        .map(|(depart, arrival)| (arrival - depart).max(0.0));
    let expected_count = expected.len();

    Ok(json!({
        "case_id": case_id,
        "classification": "synthetic_only",
        "expected_vehicle_count": expected_count,
        "tripinfo_record_count": records.len(),
        "departed_vehicle_count": departed,
        "undeparted_vehicle_count": undeparted,
        "arrived_vehicle_count": arrived,
        "not_arrived_by_simulation_end_count": expected_count - arrived,
        "completion_fraction": arrived as f64 / expected_count as f64,
        "simulation_horizon_seconds": horizon,
        "arrived_vehicles_per_simulation_second": arrived as f64 / horizon as f64,
        "first_completed_trip_departure_seconds": first_departure,
        "last_arrival_time_seconds": last_arrival,
        "observed_arrival_span_seconds": observed_span,
        "completed_trip_duration_seconds": distribution(&durations),
        "completed_trip_route_length_meters": distribution(&route_lengths),
        "completed_trip_waiting_time_seconds": distribution(&waiting_times),
        "completed_trip_time_loss_seconds": distribution(&time_losses),
        "depart_delay_seconds_for_departed_vehicles": distribution(&departure_delays),
        "rerouted_vehicle_count": if reroute_observations == records.len() {
            Some(rerouted)
        } else {
            None
        },
        "reroute_attribute_observation_count": reroute_observations,
        "arrival_conditioned_metrics_only": true,
        "claim_boundary": "synthetic SUMO smoke-run measurements; not matched-capacity benchmark or field validation"
    }))
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err(
            "usage: wildfire_sumo_metrics <fixture> <export-root> <source-sha> <horizon-seconds>"
                .to_owned(),
        );
    }
    if !valid_sha(&args[3]) {
        return Err("source SHA must be a 40-character hexadecimal SHA".to_owned());
    }
    let horizon = args[4]
        .parse::<u64>()
        .map_err(|_| "invalid simulation horizon".to_owned())?;
    if horizon == 0 {
        return Err("simulation horizon must be positive".to_owned());
    }

    let fixture_bytes = read(Path::new(&args[1]))?;
    let fixture = parse_json(&fixture_bytes, "fixture")?;
    if field(&fixture, "benchmark_id", "fixture")? != "WILDFIRE-NETWORK-001"
        || field(&fixture, "classification", "fixture")? != "synthetic_only"
    {
        return Err("fixture identity mismatch".to_owned());
    }
    let cases = array(&fixture, "cases", "fixture")?;
    if cases.len() != CASES.len() {
        return Err("expected six frozen cases".to_owned());
    }

    let root = PathBuf::from(&args[2]);
    let verification_bytes = read(&root.join("sumo-verification-receipt.json"))?;
    let verification = parse_json(&verification_bytes, "SUMO verification receipt")?;
    if field(&verification, "result", "verification")? != "pass"
        || field(&verification, "source_commit", "verification")? != args[3]
        || field(&verification, "fixture_sha256", "verification")? != hash(&fixture_bytes)
        || field(&verification, "sumo_version", "verification")? != SUMO_VERSION
    {
        return Err("prior SUMO verification receipt mismatch".to_owned());
    }

    let mut measurements = Vec::new();
    for (index, expected_id) in CASES.iter().enumerate() {
        let case = &cases[index];
        let case_id = field(case, "case_id", "fixture case")?;
        if case_id != *expected_id {
            return Err(format!("fixture case inventory mismatch at {index}"));
        }
        let bytes = read(&root.join(case_id).join("tripinfo.xml"))?;
        let xml = String::from_utf8(bytes)
            .map_err(|_| format!("{case_id}: tripinfo output is not UTF-8"))?;
        measurements.push(measure_case(case, case_id, &xml, horizon)?);
    }

    let expected_total: u64 = measurements
        .iter()
        .map(|case| case["expected_vehicle_count"].as_u64().unwrap_or(0))
        .sum();
    let arrived_total: u64 = measurements
        .iter()
        .map(|case| case["arrived_vehicle_count"].as_u64().unwrap_or(0))
        .sum();
    let mut receipt = json!({
        "result": "pass",
        "metrics_version": VERSION,
        "benchmark_id": "WILDFIRE-SUMO-TRIP-METRICS-001",
        "source_commit": args[3],
        "fixture_sha256": hash(&fixture_bytes),
        "sumo_verification_receipt_sha256": hash(&verification_bytes),
        "sumo_version": SUMO_VERSION,
        "simulation_horizon_seconds": horizon,
        "scenario_count": measurements.len(),
        "expected_vehicle_total": expected_total,
        "arrived_vehicle_total": arrived_total,
        "not_arrived_by_simulation_end_total": expected_total - arrived_total,
        "overall_completion_fraction": if expected_total > 0 {
            Some(arrived_total as f64 / expected_total as f64)
        } else {
            None
        },
        "cases": measurements,
        "metric_definitions": {
            "percentiles": "nearest-rank: rank ceil(p*n), one-indexed",
            "travel_times": "conditioned on arrivals by simulation end; inspect completion first",
            "observed_arrival_span": "last arrival minus first completed-trip departure; not necessarily source-model clearance time",
            "throughput": "arrived vehicles per configured simulation second; not calibrated road capacity",
            "not_arrived": "vehicles without nonnegative arrival by horizon; not equivalent to source-model stranded/unserved demand"
        },
        "claim_boundary": "synthetic SUMO measurement only; semantic mismatches remain, so no traffic-equivalence, SOTA, accuracy or safety claim"
    });
    receipt["receipt_sha256"] = json!(hash(
        serde_json::to_string(&receipt)
            .map_err(|error| format!("serialize receipt for digest: {error}"))?
            .as_bytes()
    ));
    let output = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("serialize metrics receipt: {error}"))?;
    fs::write(root.join("sumo-tripinfo-metrics-receipt.json"), &output)
        .map_err(|error| format!("write metrics receipt: {error}"))?;
    println!("{}", String::from_utf8_lossy(&output));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SUMO metrics failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_case() -> Value {
        json!({
            "demands": [
                {"id": "d1", "vehicles": 2},
                {"id": "d2", "vehicles": 1}
            ]
        })
    }

    #[test]
    fn attributes_allow_whitespace_and_single_quotes() {
        let parsed = parse_attributes(" id='veh' depart = \"0.00\" ").unwrap();
        assert_eq!(parsed["id"], "veh");
        assert_eq!(parsed["depart"], "0.00");
    }

    #[test]
    fn malformed_or_duplicate_attributes_fail_closed() {
        assert!(parse_attributes("id=\"a\" id=\"b\"").is_err());
        assert!(parse_attributes("id=a").is_err());
        assert!(parse_attributes("id=\"a").is_err());
    }

    #[test]
    fn nearest_rank_percentiles_are_deterministic() {
        let values: Vec<f64> = (1..=20).map(f64::from).collect();
        assert_eq!(percentile(&values, 0.50), Some(10.0));
        assert_eq!(percentile(&values, 0.90), Some(18.0));
        assert_eq!(percentile(&values, 0.95), Some(19.0));
        assert_eq!(percentile(&[], 0.95), None);
    }

    #[test]
    fn completed_and_undeparted_trips_reconcile() {
        let xml = r#"<?xml version="1.0"?><tripinfos>
        <tripinfo id="d1_00000" depart="0" arrival="10" duration="10" routeLength="100" waitingTime="1" timeLoss="2" departDelay="0" rerouteNo="0"/>
        <tripinfo id="d1_00001" depart="1" arrival="15" duration="14" routeLength="120" waitingTime="3" timeLoss="4" departDelay="0" rerouteNo="1"/>
        <tripinfo id="d2_00000" depart="-1" arrival="-1" duration="0" routeLength="0" waitingTime="0" timeLoss="0" departDelay="5" rerouteNo="0"/>
        </tripinfos>"#;
        let metrics = measure_case(&tiny_case(), "TINY", xml, 100).unwrap();
        assert_eq!(metrics["expected_vehicle_count"], 3);
        assert_eq!(metrics["arrived_vehicle_count"], 2);
        assert_eq!(metrics["undeparted_vehicle_count"], 1);
        assert_eq!(metrics["not_arrived_by_simulation_end_count"], 1);
        assert_eq!(
            metrics["completed_trip_duration_seconds"]["p95_nearest_rank"],
            14.0
        );
        assert_eq!(metrics["rerouted_vehicle_count"], 1);
    }

    #[test]
    fn duplicate_unexpected_missing_and_incomplete_records_fail() {
        let one = r#"<tripinfo id="d1_00000" depart="0" arrival="1" duration="1" routeLength="1" waitingTime="0" timeLoss="0"/>"#;
        let duplicate = format!("<?xml version=\"1.0\"?><tripinfos>{one}{one}</tripinfos>");
        assert!(measure_case(&tiny_case(), "TINY", &duplicate, 10).is_err());
        assert!(
            measure_case(
                &tiny_case(),
                "TINY",
                &format!("<?xml version=\"1.0\"?><tripinfos>{one}</tripinfos>"),
                10
            )
            .is_err()
        );
        assert!(tripinfo_records("<tripinfos><tripinfo id=\"bad\">").is_err());
        let nan = r#"<?xml version="1.0"?><tripinfos><tripinfo id="d1_00000" depart="0" arrival="NaN" duration="1" routeLength="1" waitingTime="0" timeLoss="0"/></tripinfos>"#;
        assert!(measure_case(&tiny_case(), "TINY", nan, 10).is_err());
    }
}
