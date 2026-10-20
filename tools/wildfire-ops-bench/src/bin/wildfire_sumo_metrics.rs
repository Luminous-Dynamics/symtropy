use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, path::{Path, PathBuf}, process::ExitCode};

const VERSION: &str = "0.1.0";
const SUMO_VERSION: &str = "1.27.1";
const CASES: [&str; 6] = [
    "TWO_ORIGINS_SHARED_BOTTLENECK_001", "FINITE_STORAGE_BACKPRESSURE_002",
    "ENTRY_CLOSURE_DURING_DEPARTURES_003", "ROUND_ROBIN_MERGE_FAIRNESS_004",
    "TOTAL_SHELTER_CAPACITY_HARD_GATE_005", "ACCESSIBLE_CAPACITY_HARD_GATE_006",
];
type Check<T> = Result<T, String>;

fn hash(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn read(path: &Path) -> Check<Vec<u8>> { fs::read(path).map_err(|e| format!("read {}: {e}", path.display())) }
fn parse_json(bytes: &[u8], label: &str) -> Check<Value> { serde_json::from_slice(bytes).map_err(|e| format!("parse {label}: {e}")) }
fn field<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    v.get(key).and_then(Value::as_str).ok_or_else(|| format!("{at}.{key}: expected string"))
}
fn array<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a [Value]> {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).ok_or_else(|| format!("{at}.{key}: expected array"))
}
fn uint(v: &Value, key: &str, at: &str) -> Check<u64> {
    v.get(key).and_then(Value::as_u64).ok_or_else(|| format!("{at}.{key}: expected unsigned integer"))
}
fn sha_ok(s: &str) -> bool { s.len() == 40 && s.bytes().all(|b| b.is_ascii_hexdigit()) }

fn attrs(body: &str) -> Check<BTreeMap<String, String>> {
    let b = body.as_bytes();
    let (mut i, mut out) = (0usize, BTreeMap::new());
    while i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() { i += 1; }
        if i == b.len() || b[i] == b'/' { break; }
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'=' && b[i] != b'/' { i += 1; }
        if start == i { return Err("empty XML attribute".into()); }
        let key = std::str::from_utf8(&b[start..i]).map_err(|_| "invalid attribute name UTF-8")?.to_owned();
        while i < b.len() && b[i].is_ascii_whitespace() { i += 1; }
        if i == b.len() || b[i] != b'=' { return Err(format!("{key}: missing '='")); }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() { i += 1; }
        if i == b.len() || (b[i] != b'"' && b[i] != b'\'') { return Err(format!("{key}: unquoted value")); }
        let q = b[i]; i += 1;
        let start = i;
        while i < b.len() && b[i] != q { i += 1; }
        if i == b.len() { return Err(format!("{key}: unterminated value")); }
        let value = std::str::from_utf8(&b[start..i]).map_err(|_| format!("{key}: invalid value UTF-8"))?.to_owned();
        i += 1;
        if out.insert(key.clone(), value).is_some() { return Err(format!("duplicate attribute {key}")); }
    }
    Ok(out)
}
fn trips(xml: &str) -> Check<Vec<BTreeMap<String, String>>> {
    if !xml.starts_with("<?xml") || !xml.contains("</tripinfos>") { return Err("incomplete tripinfo XML".into()); }
    let mut out = Vec::new();
    for piece in xml.split("<tripinfo ").skip(1) {
        let (body, _) = piece.split_once('>').ok_or("unterminated tripinfo tag")?;
        out.push(attrs(body)?);
    }
    Ok(out)
}
fn num(a: &BTreeMap<String, String>, key: &str) -> Check<Option<f64>> {
    let Some(s) = a.get(key) else { return Ok(None) };
    let n = s.parse::<f64>().map_err(|_| format!("tripinfo.{key}: invalid number {s}"))?;
    if !n.is_finite() { return Err(format!("tripinfo.{key}: non-finite")); }
    Ok(Some(n))
}
fn required(a: &BTreeMap<String, String>, key: &str) -> Check<f64> {
    num(a, key)?.ok_or_else(|| format!("tripinfo missing {key}"))
}
fn pct(v: &[f64], p: f64) -> Option<f64> {
    if v.is_empty() { return None; }
    let mut s = v.to_vec(); s.sort_by(f64::total_cmp);
    Some(s[((p * s.len() as f64).ceil() as usize).saturating_sub(1).min(s.len()-1)])
}
fn stats(v: &[f64]) -> Value {
    json!({"count":v.len(),"mean":if v.is_empty(){None}else{Some(v.iter().sum::<f64>()/v.len() as f64)},
        "p50_nearest_rank":pct(v,0.50),"p90_nearest_rank":pct(v,0.90),"p95_nearest_rank":pct(v,0.95),
        "max":v.iter().copied().max_by(f64::total_cmp)})
}
fn expected_ids(case: &Value, id: &str) -> Check<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for d in array(case, "demands", id)? {
        let demand = field(d, "id", id)?;
        let n = uint(d, "vehicles", demand)?;
        if n > 50_000 { return Err(format!("{id}: demand exceeds guardrail")); }
        for i in 0..n {
            let v = format!("{demand}_{i:05}");
            if !ids.insert(v.clone()) { return Err(format!("{id}: duplicate fixture vehicle {v}")); }
        }
    }
    Ok(ids)
}

fn measure(case: &Value, id: &str, xml: &str, horizon: u64) -> Check<Value> {
    let expected = expected_ids(case, id)?;
    let records = trips(xml)?;
    let mut seen = BTreeSet::new();
    let (mut departed, mut undeparted, mut arrived, mut reroute_seen, mut rerouted) = (0usize,0usize,0usize,0usize,0usize);
    let (mut depart, mut arrival, mut duration, mut length, mut waiting, mut loss, mut delays) = (Vec::new(),Vec::new(),Vec::new(),Vec::new(),Vec::new(),Vec::new(),Vec::new());
    for a in &records {
        let vid = a.get("id").ok_or_else(|| format!("{id}: record without ID"))?;
        if !expected.contains(vid) { return Err(format!("{id}: unexpected vehicle {vid}")); }
        if !seen.insert(vid.clone()) { return Err(format!("{id}: duplicate vehicle {vid}")); }
        let d = num(a,"depart")?;
        let ar = num(a,"arrival")?;
        if d.is_some_and(|x| x >= 0.0) {
            departed += 1;
            if let Some(x) = num(a,"departDelay")? { if x >= 0.0 { delays.push(x); } }
        } else { undeparted += 1; }
        if let Some(x) = a.get("rerouteNo") {
            let n = x.parse::<u64>().map_err(|_| format!("{id}: invalid rerouteNo"))?;
            reroute_seen += 1; if n > 0 { rerouted += 1; }
        }
        if ar.is_some_and(|x| x >= 0.0) {
            let d = d.filter(|x| *x >= 0.0).ok_or_else(|| format!("{id}: arrived trip missing departure"))?;
            let ar = ar.unwrap();
            let du = required(a,"duration")?; let le = required(a,"routeLength")?;
            let wa = required(a,"waitingTime")?; let lo = required(a,"timeLoss")?;
            if ar < d || [du,le,wa,lo].iter().any(|x| *x < 0.0) { return Err(format!("{id}: invalid metrics for {vid}")); }
            arrived += 1; depart.push(d); arrival.push(ar); duration.push(du); length.push(le); waiting.push(wa); loss.push(lo);
        }
    }
    if seen != expected {
        let missing: Vec<_> = expected.difference(&seen).take(5).collect();
        return Err(format!("{id}: expected {} IDs, found {}; missing examples {missing:?}", expected.len(), seen.len()));
    }
    if records.len() != expected.len() { return Err(format!("{id}: record inventory mismatch")); }
    let first = depart.iter().copied().min_by(f64::total_cmp);
    let last = arrival.iter().copied().max_by(f64::total_cmp);
    Ok(json!({
        "case_id":id,"classification":"synthetic_only","expected_vehicle_count":expected.len(),"tripinfo_record_count":records.len(),
        "departed_vehicle_count":departed,"undeparted_vehicle_count":undeparted,"arrived_vehicle_count":arrived,
        "not_arrived_by_simulation_end_count":expected.len()-arrived,"completion_fraction":arrived as f64/expected.len() as f64,
        "simulation_horizon_seconds":horizon,"arrived_vehicles_per_simulation_second":arrived as f64/horizon as f64,
        "first_completed_trip_departure_seconds":first,"last_arrival_time_seconds":last,
        "observed_arrival_span_seconds":first.zip(last).map(|(d,a)|(a-d).max(0.0)),
        "completed_trip_duration_seconds":stats(&duration),"completed_trip_route_length_meters":stats(&length),
        "completed_trip_waiting_time_seconds":stats(&waiting),"completed_trip_time_loss_seconds":stats(&loss),
        "depart_delay_seconds_for_departed_vehicles":stats(&delays),
        "rerouted_vehicle_count":if reroute_seen==records.len(){Some(rerouted)}else{None},
        "reroute_attribute_observation_count":reroute_seen,"arrival_conditioned_metrics_only":true,
        "claim_boundary":"synthetic SUMO smoke-run measurements; not matched-capacity benchmark or field validation"
    }))
}
fn run() -> Check<()> {
    let args: Vec<String> = env::args().collect();
    if args.len()!=5 { return Err("usage: wildfire_sumo_metrics <fixture> <export-root> <source-sha> <horizon-seconds>".into()); }
    if !sha_ok(&args[3]) { return Err("source SHA must be 40 hexadecimal characters".into()); }
    let horizon = args[4].parse::<u64>().map_err(|_| "invalid simulation horizon")?;
    if horizon==0 { return Err("simulation horizon must be positive".into()); }
    let fixture_bytes = read(Path::new(&args[1]))?;
    let fixture = parse_json(&fixture_bytes,"fixture")?;
    if field(&fixture,"benchmark_id","fixture")?!="WILDFIRE-NETWORK-001" || field(&fixture,"classification","fixture")?!="synthetic_only" {
        return Err("fixture identity mismatch".into());
    }
    let cases = array(&fixture,"cases","fixture")?;
    if cases.len()!=CASES.len() { return Err("expected six frozen cases".into()); }
    let root = PathBuf::from(&args[2]);
    let vr_bytes = read(&root.join("sumo-verification-receipt.json"))?;
    let vr = parse_json(&vr_bytes,"verification receipt")?;
    if field(&vr,"result","verification")?!="pass" || field(&vr,"source_commit","verification")?!=args[3]
        || field(&vr,"fixture_sha256","verification")?!=hash(&fixture_bytes) || field(&vr,"sumo_version","verification")?!=SUMO_VERSION {
        return Err("prior SUMO verification receipt mismatch".into());
    }
    let mut measurements = Vec::new();
    for (i, expected_id) in CASES.iter().enumerate() {
        let case = &cases[i]; let id = field(case,"case_id","case")?;
        if id!=*expected_id { return Err(format!("case inventory mismatch at {i}")); }
        let bytes = read(&root.join(id).join("tripinfo.xml"))?;
        measurements.push(measure(case,id,&String::from_utf8(bytes).map_err(|_|format!("{id}: tripinfo not UTF-8"))?,horizon)?);
    }
    let total: u64 = measurements.iter().map(|m|m["expected_vehicle_count"].as_u64().unwrap_or(0)).sum();
    let arrived: u64 = measurements.iter().map(|m|m["arrived_vehicle_count"].as_u64().unwrap_or(0)).sum();
    let mut receipt=json!({
        "result":"pass","metrics_version":VERSION,"benchmark_id":"WILDFIRE-SUMO-TRIP-METRICS-001",
        "source_commit":args[3],"fixture_sha256":hash(&fixture_bytes),"sumo_verification_receipt_sha256":hash(&vr_bytes),
        "sumo_version":SUMO_VERSION,"simulation_horizon_seconds":horizon,"scenario_count":measurements.len(),
        "expected_vehicle_total":total,"arrived_vehicle_total":arrived,"not_arrived_by_simulation_end_total":total-arrived,
        "overall_completion_fraction":if total>0{Some(arrived as f64/total as f64)}else{None},"cases":measurements,
        "metric_definitions":{"percentiles":"nearest-rank, rank ceil(p*n), one-indexed",
        "travel_times":"conditioned on trips that arrived by simulation end; inspect completion first",
        "observed_arrival_span":"last arrival minus first completed-trip departure; not necessarily source-model clearance time",
        "throughput":"arrived vehicles per configured simulation second; not calibrated road capacity",
        "not_arrived":"vehicles with no nonnegative arrival by horizon; not equivalent to source-model stranded/unserved demand"},
        "claim_boundary":"synthetic SUMO measurement only; semantic mismatches remain, so no traffic-equivalence, SOTA, accuracy or safety claim"
    });
    receipt["receipt_sha256"]=json!(hash(serde_json::to_string(&receipt).map_err(|e|format!("serialize receipt: {e}"))?.as_bytes()));
    let out=serde_json::to_vec_pretty(&receipt).map_err(|e|format!("serialize output: {e}"))?;
    fs::write(root.join("sumo-tripinfo-metrics-receipt.json"),&out).map_err(|e|format!("write metrics receipt: {e}"))?;
    println!("{}",String::from_utf8_lossy(&out)); Ok(())
}
fn main()->ExitCode { match run(){Ok(())=>ExitCode::SUCCESS,Err(e)=>{eprintln!("SUMO metrics failed: {e}");ExitCode::FAILURE}} }

#[cfg(test)]
mod tests {
    use super::*;
    fn case()->Value{json!({"demands":[{"id":"d1","vehicles":2},{"id":"d2","vehicles":1}]})}
    #[test] fn attributes_allow_whitespace_and_single_quotes(){
        let a=attrs(" id='veh' depart = \"0.00\" ").unwrap(); assert_eq!(a["id"],"veh"); assert_eq!(a["depart"],"0.00");
    }
    #[test] fn malformed_or_duplicate_attributes_fail_closed(){
        assert!(attrs("id=\"a\" id=\"b\"").is_err()); assert!(attrs("id=a").is_err()); assert!(attrs("id=\"a").is_err());
    }
    #[test] fn nearest_rank_is_deterministic(){
        let v:Vec<f64>=(1..=20).map(f64::from).collect(); assert_eq!(pct(&v,0.5),Some(10.0)); assert_eq!(pct(&v,0.9),Some(18.0)); assert_eq!(pct(&v,0.95),Some(19.0)); assert_eq!(pct(&[],0.95),None);
    }
    #[test] fn completed_and_undeparted_trips_reconcile(){
        let xml=r#"<?xml version="1.0"?><tripinfos>
        <tripinfo id="d1_00000" depart="0" arrival="10" duration="10" routeLength="100" waitingTime="1" timeLoss="2" departDelay="0" rerouteNo="0"/>
        <tripinfo id="d1_00001" depart="1" arrival="15" duration="14" routeLength="120" waitingTime="3" timeLoss="4" departDelay="0" rerouteNo="1"/>
        <tripinfo id="d2_00000" depart="-1" arrival="-1" duration="0" routeLength="0" waitingTime="0" timeLoss="0" departDelay="5" rerouteNo="0"/>
        </tripinfos>"#;
        let m=measure(&case(),"TINY",xml,100).unwrap(); assert_eq!(m["expected_vehicle_count"],3); assert_eq!(m["arrived_vehicle_count"],2);
        assert_eq!(m["undeparted_vehicle_count"],1); assert_eq!(m["not_arrived_by_simulation_end_count"],1);
        assert_eq!(m["completed_trip_duration_seconds"]["p95_nearest_rank"],14.0); assert_eq!(m["rerouted_vehicle_count"],1);
    }
    #[test] fn duplicate_unexpected_missing_and_incomplete_records_fail(){
        let one=r#"<tripinfo id="d1_00000" depart="0" arrival="1" duration="1" routeLength="1" waitingTime="0" timeLoss="0"/>"#;
        let duplicate=format!("<?xml version=\"1.0\"?><tripinfos>{one}{one}</tripinfos>");
        assert!(measure(&case(),"TINY",&duplicate,10).is_err());
        assert!(measure(&case(),"TINY",&format!("<?xml version=\"1.0\"?><tripinfos>{one}</tripinfos>"),10).is_err());
        assert!(trips("<tripinfos><tripinfo id=\"bad\">").is_err());
        let nan=r#"<?xml version="1.0"?><tripinfos><tripinfo id="d1_00000" depart="0" arrival="NaN" duration="1" routeLength="1" waitingTime="0" timeLoss="0"/></tripinfos>"#;
        assert!(measure(&case(),"TINY",nan,10).is_err());
    }
}
