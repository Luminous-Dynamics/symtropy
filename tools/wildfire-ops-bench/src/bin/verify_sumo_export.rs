use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SUMO_VERSION: &str = "1.27.1";
const CASE_IDS: [&str; 6] = [
    "TWO_ORIGINS_SHARED_BOTTLENECK_001",
    "FINITE_STORAGE_BACKPRESSURE_002",
    "ENTRY_CLOSURE_DURING_DEPARTURES_003",
    "ROUND_ROBIN_MERGE_FAIRNESS_004",
    "TOTAL_SHELTER_CAPACITY_HARD_GATE_005",
    "ACCESSIBLE_CAPACITY_HARD_GATE_006",
];
const FILES: [&str; 4] = [
    "nodes.nod.xml",
    "edges.edg.xml",
    "demand.rou.xml",
    "closures.add.xml",
];

type Check<T> = Result<T, String>;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read(path: &Path) -> Check<Vec<u8>> {
    fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))
}

fn parse(bytes: &[u8], path: &Path) -> Check<Value> {
    serde_json::from_slice(bytes).map_err(|e| format!("parse {}: {e}", path.display()))
}

fn string<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key}: expected string"))
}

fn array<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    v.get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{at}.{key}: expected array"))
}

fn sha_format(s: &str) -> bool {
    s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn xml_start_tags<'a>(xml: &'a str, name: &str) -> Check<Vec<&'a str>> {
    let needle = format!("<{name}");
    let bytes = xml.as_bytes();
    let mut cursor = 0usize;
    let mut tags = Vec::new();
    while let Some(relative) = xml[cursor..].find(&needle) {
        let start = cursor + relative;
        let after_name = start + needle.len();
        let boundary = bytes.get(after_name).copied();
        if !matches!(boundary, Some(b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>')) {
            cursor = after_name;
            continue;
        }
        let end = xml[after_name..]
            .find('>')
            .map(|offset| after_name + offset)
            .ok_or_else(|| format!("unterminated <{name} ...> element"))?;
        tags.push(&xml[start..=end]);
        cursor = end + 1;
    }
    Ok(tags)
}

fn xml_attributes(tag: &str) -> Check<BTreeMap<String, String>> {
    let bytes = tag.as_bytes();
    if bytes.first() != Some(&b'<') {
        return Err("XML element does not start with '<'".to_owned());
    }
    let mut cursor = 1usize;
    while cursor < bytes.len()
        && !matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n' | b'/' | b'>')
    {
        cursor += 1;
    }
    let mut attributes = BTreeMap::new();
    loop {
        while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n' | b'/') {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'>' {
            break;
        }
        let key_start = cursor;
        while cursor < bytes.len()
            && !matches!(
                bytes[cursor],
                b' ' | b'\t' | b'\r' | b'\n' | b'=' | b'/' | b'>'
            )
        {
            cursor += 1;
        }
        if cursor == key_start {
            return Err(format!("malformed XML attribute in {tag}"));
        }
        let key = std::str::from_utf8(&bytes[key_start..cursor])
            .map_err(|_| "XML attribute name is not UTF-8".to_owned())?
            .to_owned();
        while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            return Err(format!("XML attribute {key} lacks '='"));
        }
        cursor += 1;
        while cursor < bytes.len() && matches!(bytes[cursor], b' ' | b'\t' | b'\r' | b'\n') {
            cursor += 1;
        }
        let quote = *bytes
            .get(cursor)
            .ok_or_else(|| format!("XML attribute {key} has no value"))?;
        if quote != b'"' && quote != b'\'' {
            return Err(format!("XML attribute {key} is not quoted"));
        }
        cursor += 1;
        let value_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != quote {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            return Err(format!("XML attribute {key} has an unterminated value"));
        }
        let value = std::str::from_utf8(&bytes[value_start..cursor])
            .map_err(|_| format!("XML attribute {key} value is not UTF-8"))?
            .to_owned();
        cursor += 1;
        if attributes.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate XML attribute {key}"));
        }
    }
    Ok(attributes)
}

fn required_finite_nonnegative(
    attributes: &BTreeMap<String, String>,
    key: &str,
    vehicle_id: &str,
) -> Check<f64> {
    let raw = attributes
        .get(key)
        .ok_or_else(|| format!("tripinfo {vehicle_id} lacks {key}"))?;
    let value = raw
        .parse::<f64>()
        .map_err(|_| format!("tripinfo {vehicle_id}.{key} is not numeric"))?;
    if !value.is_finite() || value < 0.0 {
        return Err(format!(
            "tripinfo {vehicle_id}.{key} must be finite and non-negative"
        ));
    }
    Ok(value)
}

fn metric_summary(values: &[f64]) -> Value {
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let quantile = |p: f64| -> Value {
        if sorted.is_empty() {
            Value::Null
        } else {
            let rank = ((p * sorted.len() as f64).ceil() as usize)
                .max(1)
                .min(sorted.len());
            json!(sorted[rank - 1])
        }
    };
    let mean = if sorted.is_empty() {
        Value::Null
    } else {
        json!(sorted.iter().sum::<f64>() / sorted.len() as f64)
    };
    json!({
        "count": sorted.len(),
        "mean": mean,
        "p50_nearest_rank": quantile(0.50),
        "p90_nearest_rank": quantile(0.90),
        "p95_nearest_rank": quantile(0.95),
        "max": sorted.last().copied()
    })
}

fn expected_vehicle_ids(demands: &[Value], at: &str) -> Check<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for demand in demands {
        let demand_id = string(demand, "id", at)?;
        let count = demand
            .get("vehicles")
            .and_then(Value::as_u64)
            .ok_or_else(|| format!("{at}.{demand_id}.vehicles: expected non-negative integer"))?;
        for number in 0..count {
            ids.insert(format!("{demand_id}_{number:05}"));
        }
    }
    Ok(ids)
}

fn parse_sumo_outcomes(xml: &str, expected_ids: &BTreeSet<String>) -> Check<Value> {
    let tags = xml_start_tags(xml, "tripinfo")?;
    let mut seen_ids = BTreeSet::new();
    let mut duration = Vec::new();
    let mut waiting = Vec::new();
    let mut time_loss = Vec::new();
    let mut depart_delay = Vec::new();
    let mut route_length = Vec::new();
    let mut arrived = 0usize;
    let mut not_arrived = 0usize;

    for tag in &tags {
        let attrs = xml_attributes(tag)?;
        let id = attrs
            .get("id")
            .ok_or_else(|| "tripinfo element lacks id".to_owned())?;
        if !seen_ids.insert(id.clone()) {
            return Err(format!("duplicate tripinfo vehicle ID {id}"));
        }
        if !expected_ids.contains(id) {
            return Err(format!("tripinfo contains unplanned vehicle ID {id}"));
        }
        let arrival_raw = attrs
            .get("arrival")
            .ok_or_else(|| format!("tripinfo {id} lacks arrival"))?;
        let arrival = arrival_raw
            .parse::<f64>()
            .map_err(|_| format!("tripinfo {id}.arrival is not numeric"))?;
        if !arrival.is_finite() {
            return Err(format!("tripinfo {id}.arrival is not finite"));
        }
        if arrival < 0.0 {
            not_arrived += 1;
            continue;
        }
        arrived += 1;
        duration.push(required_finite_nonnegative(&attrs, "duration", id)?);
        waiting.push(required_finite_nonnegative(&attrs, "waitingTime", id)?);
        time_loss.push(required_finite_nonnegative(&attrs, "timeLoss", id)?);
        depart_delay.push(required_finite_nonnegative(&attrs, "departDelay", id)?);
        route_length.push(required_finite_nonnegative(&attrs, "routeLength", id)?);
    }

    let missing: Vec<&String> = expected_ids.difference(&seen_ids).collect();
    if !missing.is_empty() {
        return Err(format!(
            "tripinfo coverage mismatch: {} planned vehicles lack records (first: {})",
            missing.len(),
            missing[0]
        ));
    }
    if tags.len() != expected_ids.len() {
        return Err(format!(
            "tripinfo record count {} differs from planned vehicle count {}",
            tags.len(),
            expected_ids.len()
        ));
    }
    let planned = expected_ids.len();
    Ok(json!({
        "planned_vehicle_count": planned,
        "tripinfo_record_count": tags.len(),
        "arrived_by_horizon_vehicle_count": arrived,
        "not_arrived_by_horizon_vehicle_count": not_arrived,
        "arrival_fraction": if planned == 0 { 0.0 } else { arrived as f64 / planned as f64 },
        "quantile_method": "nearest-rank; computed over arrived trips only",
        "trip_duration_seconds": metric_summary(&duration),
        "trip_waiting_seconds": metric_summary(&waiting),
        "trip_time_loss_seconds": metric_summary(&time_loss),
        "trip_departure_delay_seconds": metric_summary(&depart_delay),
        "trip_route_length_m": metric_summary(&route_length),
        "claim_boundary": "SUMO-only outcomes under the pinned synthetic adapter; not equivalent to Symtropy model outcomes"
    }))
}

fn semantic_comparability(case: &Value, manifest: &Value, at: &str) -> Check<Value> {
    let edges = array(case, "edges", at)?;
    let sinks = array(case, "sinks", at)?;
    let demands = array(case, "demands", at)?;
    let gaps = array(manifest, "semantic_gaps", at)?;
    let mut reasons = Vec::<Value>::new();

    let gap_detail = |needle: &str| -> Check<String> {
        gaps.iter()
            .filter_map(Value::as_str)
            .find(|gap| gap.contains(needle))
            .map(str::to_owned)
            .ok_or_else(|| {
                format!(
                    "{at}: manifest omitted required semantic-gap declaration containing {needle:?}"
                )
            })
    };

    let edge_limits_declared = edges
        .iter()
        .any(|edge| edge.get("capacity_per_step").is_some() || edge.get("storage").is_some());
    if edge_limits_declared {
        reasons.push(json!({
            "code": "EDGE_CAPACITY_STORAGE_UNCALIBRATED",
            "severity": "blocking",
            "detail": gap_detail("Abstract vehicles-per-step capacities and finite storage")?
        }));
    }

    let sink_limits_declared = sinks.iter().any(|sink| {
        sink.get("ingress_capacity_per_step").is_some()
            || sink.get("total_capacity").is_some()
            || sink.get("accessible_capacity").is_some()
    });
    if sink_limits_declared {
        reasons.push(json!({
            "code": "SINK_SERVICE_GATES_NOT_ENFORCED",
            "severity": "blocking",
            "detail": gap_detail("Sink ingress and total/accessibility constraints")?
        }));
    }

    let accessible_demand = demands.iter().any(|demand| {
        demand
            .get("accessible_vehicles")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            > 0
    });
    if accessible_demand {
        reasons.push(json!({
            "code": "ACCESSIBILITY_LABEL_ONLY",
            "severity": "blocking",
            "detail": gap_detail("Accessible vehicle type is only a parameter label")?
        }));
    }

    let active_closure = edges.iter().any(|edge| {
        edge.get("closed_from_step")
            .is_some_and(|value| !value.is_null())
    });
    if active_closure {
        reasons.push(json!({
            "code": "CLOSURE_POLICY_DIFFERS",
            "severity": "blocking",
            "detail": gap_detail("SUMO closingReroute may change a vehicle's route")?
        }));
    }

    let status = if reasons.is_empty() {
        "comparable"
    } else {
        "non_comparable"
    };
    Ok(json!({
        "status": status,
        "comparison_permitted": reasons.is_empty(),
        "blocking_reasons": reasons,
        "classification_rule": "cross-model comparison is permitted only when every applicable semantic gap is closed or explicitly mapped and validated"
    }))
}

fn verify(fixture_bytes: &[u8], root: &Path, source_sha: &str) -> Check<Value> {
    if !sha_format(source_sha) {
        return Err("expected source SHA must contain 40 hexadecimal characters".to_owned());
    }
    let fixture = parse(fixture_bytes, Path::new("<fixture>"))?;
    if fixture.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-NETWORK-001")
        || fixture.get("classification").and_then(Value::as_str) != Some("synthetic_only")
    {
        return Err("fixture benchmark ID or classification mismatch".to_owned());
    }
    let cases = array(&fixture, "cases", "fixture")?;
    if cases.len() != CASE_IDS.len() {
        return Err("fixture must have six cases".to_owned());
    }
    for (i, case) in cases.iter().enumerate() {
        if string(case, "case_id", "fixture case")? != CASE_IDS[i] {
            return Err(format!("fixture case inventory/order mismatch at {i}"));
        }
    }

    let index_path = root.join("index.json");
    let index_bytes = read(&index_path)?;
    let index = parse(&index_bytes, &index_path)?;
    if index.get("result").and_then(Value::as_str) != Some("pass")
        || index.get("source_commit").and_then(Value::as_str) != Some(source_sha)
        || index.get("fixture_sha256").and_then(Value::as_str) != Some(hash(fixture_bytes).as_str())
        || index.get("scenario_count").and_then(Value::as_u64) != Some(CASE_IDS.len() as u64)
    {
        return Err(
            "index metadata, source SHA, fixture digest, or case count mismatch".to_owned(),
        );
    }
    let indexed = array(&index, "cases", "index")?;
    if indexed.len() != CASE_IDS.len() {
        return Err("index must contain six cases".to_owned());
    }

    let netconvert_path = root.join("netconvert-version.txt");
    let sumo_path = root.join("sumo-version.txt");
    let netconvert_version = String::from_utf8(read(&netconvert_path)?)
        .map_err(|_| "netconvert version is not UTF-8".to_owned())?;
    let sumo_version =
        String::from_utf8(read(&sumo_path)?).map_err(|_| "SUMO version is not UTF-8".to_owned())?;
    if !netconvert_version.contains(SUMO_VERSION) || !sumo_version.contains(SUMO_VERSION) {
        return Err(format!("runtime did not report pinned {SUMO_VERSION}"));
    }

    let mut receipts = Vec::new();
    for i in 0..CASE_IDS.len() {
        let case_id = CASE_IDS[i];
        let case = &cases[i];
        let row = &indexed[i];
        let at = format!("case[{case_id}]");
        if string(row, "case_id", &at)? != case_id || string(row, "directory", &at)? != case_id {
            return Err(format!("{at}: index case identity mismatch"));
        }
        let dir = root.join(case_id);
        let manifest_path = dir.join("manifest.json");
        let manifest_bytes = read(&manifest_path)?;
        let manifest_sha = hash(&manifest_bytes);
        if string(row, "manifest_sha256", &at)? != manifest_sha {
            return Err(format!("{at}: manifest SHA mismatch"));
        }
        let manifest = parse(&manifest_bytes, &manifest_path)?;
        if manifest.get("case_id").and_then(Value::as_str) != Some(case_id)
            || manifest.get("result").and_then(Value::as_str) != Some("pass")
            || manifest.get("claim_boundary").and_then(Value::as_str)
                != Some("structural export only; no matched SUMO simulation was executed")
        {
            return Err(format!("{at}: manifest metadata/claim boundary mismatch"));
        }
        let manifest_files = manifest
            .get("files_sha256")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{at}: missing manifest files_sha256"))?;
        let index_files = row
            .get("file_sha256")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{at}: missing index file_sha256"))?;
        let mut verified_files = serde_json::Map::new();
        for file in FILES {
            let bytes = read(&dir.join(file))?;
            let digest = hash(&bytes);
            if manifest_files.get(file).and_then(Value::as_str) != Some(digest.as_str())
                || index_files.get(file).and_then(Value::as_str) != Some(digest.as_str())
            {
                return Err(format!("{at}: file digest mismatch for {file}"));
            }
            let xml = String::from_utf8(bytes).map_err(|_| format!("{at}: {file} is not UTF-8"))?;
            if !xml.starts_with("<?xml") || !xml.contains("</") {
                return Err(format!("{at}: {file} does not resemble an XML document"));
            }
            verified_files.insert(file.to_owned(), json!(digest));
        }

        let cfg_edges = array(case, "edges", &at)?;
        let cfg_demands = array(case, "demands", &at)?;
        let cfg_sinks = array(case, "sinks", &at)?;
        let nodes_xml = String::from_utf8(read(&dir.join("nodes.nod.xml"))?)
            .map_err(|_| format!("{at}: nodes XML is not UTF-8"))?;
        let edges_xml = String::from_utf8(read(&dir.join("edges.edg.xml"))?)
            .map_err(|_| format!("{at}: edges XML is not UTF-8"))?;
        let routes_xml = String::from_utf8(read(&dir.join("demand.rou.xml"))?)
            .map_err(|_| format!("{at}: routes XML is not UTF-8"))?;
        let mut expected_nodes = BTreeSet::new();
        for edge in cfg_edges {
            let id = string(edge, "id", &at)?;
            let from = string(edge, "from", &at)?;
            let to = string(edge, "to", &at)?;
            if !edges_xml.contains(&format!("id=\"{id}\""))
                || !edges_xml.contains(&format!("from=\"{from}\""))
                || !edges_xml.contains(&format!("to=\"{to}\""))
            {
                return Err(format!("{at}: missing exported edge/endpoints for {id}"));
            }
            expected_nodes.insert(from.to_owned());
            expected_nodes.insert(to.to_owned());
        }
        for node in &expected_nodes {
            if !nodes_xml.contains(&format!("id=\"{node}\"")) {
                return Err(format!("{at}: missing node {node}"));
            }
        }
        let expected_vehicles: u64 = cfg_demands.iter().try_fold(0u64, |sum, demand| {
            let count = demand
                .get("vehicles")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{at}: invalid vehicle count"))?;
            sum.checked_add(count)
                .ok_or_else(|| format!("{at}: vehicle count overflow"))
        })?;
        let exported_vehicles = routes_xml.matches("<vehicle id=").count() as u64;
        if exported_vehicles != expected_vehicles
            || manifest.get("exported_vehicles").and_then(Value::as_u64) != Some(expected_vehicles)
            || manifest.get("network_edges").and_then(Value::as_u64) != Some(cfg_edges.len() as u64)
            || manifest.get("demand_streams").and_then(Value::as_u64)
                != Some(cfg_demands.len() as u64)
            || manifest
                .get("sinks")
                .and_then(Value::as_array)
                .map(Vec::len)
                != Some(cfg_sinks.len())
        {
            return Err(format!(
                "{at}: exported graph/demand/sink inventory differs from fixture"
            ));
        }

        let net_path = dir.join("network.net.xml");
        let net_bytes = read(&net_path)?;
        let net_xml = String::from_utf8(net_bytes.clone())
            .map_err(|_| format!("{at}: netconvert output is not UTF-8"))?;
        if !net_xml.starts_with("<?xml") {
            return Err(format!("{at}: netconvert output is not XML"));
        }
        for edge in cfg_edges {
            let id = string(edge, "id", &at)?;
            if !net_xml.contains(&format!("id=\"{id}\"")) {
                return Err(format!("{at}: netconvert output missing source edge {id}"));
            }
        }

        let tripinfo_bytes = read(&dir.join("tripinfo.xml"))?;
        let tripinfo_xml = String::from_utf8(tripinfo_bytes.clone())
            .map_err(|_| format!("{at}: SUMO tripinfo output is not UTF-8"))?;
        if !tripinfo_xml.starts_with("<?xml") || !tripinfo_xml.contains("</tripinfos>") {
            return Err(format!(
                "{at}: SUMO tripinfo output is not a complete XML document"
            ));
        }
        let stdout_bytes = read(&dir.join("sumo-smoke.stdout.log"))?;
        let stderr_bytes = read(&dir.join("sumo-smoke.stderr.log"))?;
        let expected_ids = expected_vehicle_ids(cfg_demands, &at)?;
        if expected_ids.len() as u64 != exported_vehicles {
            return Err(format!(
                "{at}: expected vehicle-ID inventory differs from exported demand"
            ));
        }
        let sumo_behavior = parse_sumo_outcomes(&tripinfo_xml, &expected_ids)?;
        let comparability = semantic_comparability(case, &manifest, &at)?;
        receipts.push(json!({
            "case_id": case_id,
            "manifest_sha256": manifest_sha,
            "file_sha256": verified_files,
            "network_net_xml_sha256": hash(&net_bytes),
            "sumo_tripinfo_sha256": hash(&tripinfo_bytes),
            "sumo_smoke_stdout_sha256": hash(&stdout_bytes),
            "sumo_smoke_stderr_sha256": hash(&stderr_bytes),
            "sumo_smoke_completed": true,
            "tripinfo_records": tripinfo_xml.matches("<tripinfo ").count(),
            "sumo_behavior": sumo_behavior,
            "comparability": comparability,
            "source_edge_count": cfg_edges.len(),
            "source_node_count": expected_nodes.len(),
            "demand_stream_count": cfg_demands.len(),
            "sink_count": cfg_sinks.len(),
            "vehicle_count": exported_vehicles,
            "netconvert_output_contains_all_source_edges": true
        }));
    }
    let behavior_cases: Vec<Value> = receipts
        .iter()
        .map(|case| {
            json!({
                "case_id": case["case_id"],
                "sumo_behavior": case["sumo_behavior"]
            })
        })
        .collect();
    let comparability_cases: Vec<Value> = receipts
        .iter()
        .map(|case| {
            json!({
                "case_id": case["case_id"],
                "status": case["comparability"]["status"],
                "comparison_permitted": case["comparability"]["comparison_permitted"],
                "blocking_reasons": case["comparability"]["blocking_reasons"]
            })
        })
        .collect();
    let comparable_count = comparability_cases
        .iter()
        .filter(|case| case["status"].as_str() == Some("comparable"))
        .count();
    let non_comparable_count = comparability_cases.len() - comparable_count;
    let comparability_receipt = json!({
        "result": "pass",
        "benchmark_id": "WILDFIRE-SUMO-COMPARABILITY-001",
        "source_commit": source_sha,
        "fixture_sha256": hash(fixture_bytes),
        "sumo_version": SUMO_VERSION,
        "case_count": comparability_cases.len(),
        "summary": {
            "comparable_case_count": comparable_count,
            "non_comparable_case_count": non_comparable_count,
            "cross_model_comparison_permitted": non_comparable_count == 0
        },
        "cases": comparability_cases,
        "claim_boundary": "this is a semantic gate, not evidence of model accuracy; non-comparable cases must not enter aggregate cross-model scorecards"
    });
    let comparability_path = root.join("sumo-comparability-receipt.json");
    let comparability_bytes = serde_json::to_vec_pretty(&comparability_receipt)
        .map_err(|e| format!("serialize SUMO comparability receipt: {e}"))?;
    fs::write(&comparability_path, &comparability_bytes)
        .map_err(|e| format!("write {}: {e}", comparability_path.display()))?;

    let behavior_receipt = json!({
        "result": "pass",
        "benchmark_id": "WILDFIRE-SUMO-OUTCOMES-001",
        "source_commit": source_sha,
        "fixture_sha256": hash(fixture_bytes),
        "sumo_version": SUMO_VERSION,
        "case_count": CASE_IDS.len(),
        "cases": behavior_cases,
        "quantile_method": "nearest-rank",
        "claim_boundary": "quantitative SUMO-only diagnostics; synthetic adapter semantics differ from Symtropy and this is not a matched-model comparison"
    });
    let behavior_path = root.join("sumo-behavior-receipt.json");
    let behavior_bytes = serde_json::to_vec_pretty(&behavior_receipt)
        .map_err(|e| format!("serialize SUMO behavior receipt: {e}"))?;
    fs::write(&behavior_path, &behavior_bytes)
        .map_err(|e| format!("write {}: {e}", behavior_path.display()))?;

    Ok(json!({
        "result": "pass",
        "verifier_version": "0.2.0",
        "benchmark_id": "WILDFIRE-SUMO-EXPORT-001",
        "source_commit": source_sha,
        "fixture_sha256": hash(fixture_bytes),
        "export_index_sha256": hash(&index_bytes),
        "sumo_version": SUMO_VERSION,
        "netconvert_version_output": netconvert_version.trim(),
        "sumo_version_output": sumo_version.trim(),
        "case_count": CASE_IDS.len(),
        "cases": receipts,
        "claim_boundary": "independent hash/inventory/network-build verification only; not a traffic-equivalence, accuracy or safety claim"
    }))
}

fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: verify_sumo_export <fixture.json> <export-root> <expected-source-sha>"
                .to_owned(),
        );
    }
    let fixture_path = Path::new(&args[1]);
    let fixture_bytes = read(fixture_path)?;
    let receipt = verify(&fixture_bytes, Path::new(&args[2]), &args[3])?;
    let output = PathBuf::from(&args[2]).join("sumo-verification-receipt.json");
    let bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|e| format!("serialize verification receipt: {e}"))?;
    fs::write(&output, &bytes).map_err(|e| format!("write {}: {e}", output.display()))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SUMO export verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn tripinfo_metrics_cover_arrived_and_unfinished_planned_vehicles() {
        let expected = ids(&["A_00000", "A_00001", "B_00000"]);
        let xml = r#"<?xml version="1.0"?><tripinfos>
<tripinfo id="A_00000" arrival="20" duration="20" waitingTime="3" timeLoss="5" departDelay="1" routeLength="100"/>
<tripinfo id="A_00001" arrival="30" duration="30" waitingTime="8" timeLoss="9" departDelay="0" routeLength="100"/>
<tripinfo id="B_00000" arrival="-1" duration="-1" waitingTime="0" timeLoss="0" departDelay="0" routeLength="100"/>
</tripinfos>"#;
        let result = parse_sumo_outcomes(xml, &expected).expect("valid outcome report");
        assert_eq!(result["planned_vehicle_count"], 3);
        assert_eq!(result["tripinfo_record_count"], 3);
        assert_eq!(result["arrived_by_horizon_vehicle_count"], 2);
        assert_eq!(result["not_arrived_by_horizon_vehicle_count"], 1);
        assert_eq!(result["arrival_fraction"], 2.0 / 3.0);
        assert_eq!(result["trip_duration_seconds"]["p50_nearest_rank"], 20.0);
        assert_eq!(result["trip_duration_seconds"]["p95_nearest_rank"], 30.0);
        assert_eq!(result["trip_waiting_seconds"]["mean"], 5.5);
    }

    #[test]
    fn tripinfo_verification_rejects_missing_duplicate_and_unplanned_vehicle_ids() {
        let expected = ids(&["A_00000", "A_00001"]);
        let one = r#"<tripinfos><tripinfo id="A_00000" arrival="-1"/></tripinfos>"#;
        assert!(
            parse_sumo_outcomes(one, &expected)
                .unwrap_err()
                .contains("coverage mismatch")
        );

        let duplicate = r#"<tripinfos><tripinfo id="A_00000" arrival="-1"/><tripinfo id="A_00000" arrival="-1"/></tripinfos>"#;
        assert!(
            parse_sumo_outcomes(duplicate, &expected)
                .unwrap_err()
                .contains("duplicate")
        );

        let extra = r#"<tripinfos><tripinfo id="A_00000" arrival="-1"/><tripinfo id="X_00000" arrival="-1"/></tripinfos>"#;
        assert!(
            parse_sumo_outcomes(extra, &expected)
                .unwrap_err()
                .contains("unplanned")
        );
    }

    #[test]
    fn tripinfo_verification_rejects_invalid_completed_trip_metrics() {
        let expected = ids(&["A_00000"]);
        let xml = r#"<tripinfos><tripinfo id="A_00000" arrival="10" duration="NaN" waitingTime="0" timeLoss="0" departDelay="0" routeLength="1"/></tripinfos>"#;
        assert!(
            parse_sumo_outcomes(xml, &expected)
                .unwrap_err()
                .contains("must be finite and non-negative")
        );
    }

    fn fixture_for_semantic_tests() -> Value {
        serde_json::from_str(include_str!(
            "../../../../benchmarks/fixtures/WILDFIRE_NETWORK_FLOW_V0_1.json"
        ))
        .expect("frozen network fixture parses")
    }

    fn declared_semantic_gaps() -> Value {
        json!({
            "semantic_gaps": [
                "Abstract vehicles-per-step capacities and finite storage are sidecar metadata; SUMO lane/speed flow is not calibrated to enforce equivalent limits.",
                "Sink ingress and total/accessibility constraints are sidecar metadata, not enforced by this adapter in SUMO.",
                "Accessible vehicle type is only a parameter label; it is not an accessibility enforcement model.",
                "SUMO closingReroute may change a vehicle's route; the source benchmark blocks entry while retaining fixed routes."
            ]
        })
    }

    #[test]
    fn all_frozen_scenarios_are_explicitly_blocked_from_cross_model_scoring() {
        let fixture = fixture_for_semantic_tests();
        let cases = fixture["cases"].as_array().expect("cases");
        assert_eq!(cases.len(), CASE_IDS.len());
        for case in cases {
            let id = case["case_id"].as_str().expect("case ID");
            let report = semantic_comparability(case, &declared_semantic_gaps(), id)
                .expect("manifest declares every applicable gap");
            assert_eq!(report["status"], "non_comparable", "{id}");
            assert_eq!(report["comparison_permitted"], false, "{id}");
            let codes: Vec<&str> = report["blocking_reasons"]
                .as_array()
                .expect("reason array")
                .iter()
                .map(|reason| reason["code"].as_str().expect("reason code"))
                .collect();
            assert!(
                codes.contains(&"EDGE_CAPACITY_STORAGE_UNCALIBRATED"),
                "{id}"
            );
            assert!(codes.contains(&"SINK_SERVICE_GATES_NOT_ENFORCED"), "{id}");
            assert!(codes.contains(&"ACCESSIBILITY_LABEL_ONLY"), "{id}");
            if id == "ENTRY_CLOSURE_DURING_DEPARTURES_003" {
                assert!(codes.contains(&"CLOSURE_POLICY_DIFFERS"));
            } else {
                assert!(!codes.contains(&"CLOSURE_POLICY_DIFFERS"));
            }
        }
    }

    #[test]
    fn missing_semantic_gap_declaration_fails_closed() {
        let case = json!({
            "edges": [{"capacity_per_step": 2, "storage": 4}],
            "sinks": [],
            "demands": []
        });
        let manifest = json!({"semantic_gaps": []});
        assert!(
            semantic_comparability(&case, &manifest, "negative-control")
                .unwrap_err()
                .contains("omitted required semantic-gap declaration")
        );
    }

    #[test]
    fn commit_sha_requires_exactly_forty_hexadecimal_characters() {
        assert!(sha_format("bd248f34810dd042e6736d2a4b8762326cbcf8e8"));
        assert!(!sha_format("1234"));
        assert!(!sha_format("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"));
    }

    #[test]
    fn pinned_runtime_version_is_explicit() {
        assert_eq!(SUMO_VERSION, "1.27.1");
    }

    #[test]
    fn digest_commits_to_raw_bytes() {
        assert_eq!(hash(b"fixture"), hash(b"fixture"));
        assert_ne!(hash(b"fixture"), hash(b"fixture\n"));
    }
}
