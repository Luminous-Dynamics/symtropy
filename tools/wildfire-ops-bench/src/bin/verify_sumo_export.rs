use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
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
                != Some("structural export only; no matched simulation was executed")
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
            "source_edge_count": cfg_edges.len(),
            "source_node_count": expected_nodes.len(),
            "demand_stream_count": cfg_demands.len(),
            "sink_count": cfg_sinks.len(),
            "vehicle_count": exported_vehicles,
            "netconvert_output_contains_all_source_edges": true
        }));
    }
    Ok(json!({
        "result": "pass",
        "verifier_version": "0.1.0",
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
