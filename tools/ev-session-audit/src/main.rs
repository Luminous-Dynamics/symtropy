use serde::Serialize;
use serde_json::{Map, Value};
use std::{collections::BTreeMap, env, fs, process::ExitCode};

const CLAIM_CEILING: &str = "Descriptive audit of caller-declared EV charging-session exports. This is not a controller benchmark, causal analysis, battery-health estimator, certification, or proof of generalization.";

#[derive(Serialize)]
struct Distribution {
    count: usize,
    min: Option<f64>,
    p50: Option<f64>,
    p95: Option<f64>,
    max: Option<f64>,
    mean: Option<f64>,
}
#[derive(Default)]
struct Acc {
    energy: Vec<f64>,
    dwell: Vec<f64>,
    active: Vec<f64>,
    post: Vec<f64>,
    total: f64,
    missing_done: usize,
}
#[derive(Serialize)]
struct Stats {
    valid_session_count: usize,
    sessions_without_done_charging_time: usize,
    energy_delivered_kwh_total: f64,
    energy_delivered_kwh: Distribution,
    connected_dwell_minutes: Distribution,
    active_charging_minutes: Distribution,
    post_charge_occupancy_minutes: Distribution,
}
#[derive(Serialize)]
struct Invalid {
    row_index_zero_based: usize,
    reason: String,
}
#[derive(Serialize)]
struct Report {
    report_schema_version: String,
    tool: String,
    dataset_id: String,
    source_uri: String,
    evidence_class_declared_by_caller: String,
    provenance_warning: String,
    claim_ceiling: String,
    input_rows: usize,
    valid_rows: usize,
    invalid_rows: Vec<Invalid>,
    overall: Stats,
    by_site: BTreeMap<String, Stats>,
}
struct Meta {
    dataset_id: String,
    source_uri: String,
    evidence_class: String,
}
#[derive(Debug)]
struct Row {
    site: String,
    energy: f64,
    dwell: f64,
    active: Option<f64>,
    post: Option<f64>,
}

impl Acc {
    fn push(&mut self, r: &Row) {
        self.total += r.energy;
        self.energy.push(r.energy);
        self.dwell.push(r.dwell);
        match (r.active, r.post) {
            (Some(a), Some(p)) => {
                self.active.push(a);
                self.post.push(p);
            }
            _ => self.missing_done += 1,
        }
    }
    fn finish(self) -> Stats {
        Stats {
            valid_session_count: self.energy.len(),
            sessions_without_done_charging_time: self.missing_done,
            energy_delivered_kwh_total: self.total,
            energy_delivered_kwh: dist(self.energy),
            connected_dwell_minutes: dist(self.dwell),
            active_charging_minutes: dist(self.active),
            post_charge_occupancy_minutes: dist(self.post),
        }
    }
}
fn dist(mut v: Vec<f64>) -> Distribution {
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let median = if n == 0 {
        None
    } else if n % 2 == 1 {
        Some(v[n / 2])
    } else {
        Some((v[n / 2 - 1] + v[n / 2]) / 2.0)
    };
    let p95 = if n == 0 {
        None
    } else {
        Some(v[(95 * n).div_ceil(100).saturating_sub(1)])
    };
    Distribution {
        count: n,
        min: v.first().copied(),
        p50: median,
        p95,
        max: v.last().copied(),
        mean: if n == 0 {
            None
        } else {
            Some(v.iter().sum::<f64>() / n as f64)
        },
    }
}
fn sessions(v: &Value) -> Result<&[Value], String> {
    if let Some(a) = v.as_array() {
        return Ok(a);
    }
    let o = v
        .as_object()
        .ok_or("expected JSON array or response object")?;
    for key in ["_items", "sessions", "items", "data"] {
        if let Some(a) = o.get(key).and_then(Value::as_array) {
            return Ok(a);
        }
    }
    Err("no session array found (accepted keys: _items, sessions, items, data)".into())
}
fn num(o: &Map<String, Value>, k: &str) -> Result<f64, String> {
    let v = o
        .get(k)
        .filter(|x| !x.is_null())
        .ok_or_else(|| format!("{k} missing"))?;
    let n = v
        .as_f64()
        .ok_or_else(|| format!("{k} must be a JSON number"))?;
    if !n.is_finite() {
        return Err(format!("{k} must be finite"));
    }
    Ok(n)
}
fn time(o: &Map<String, Value>, k: &str, required: bool) -> Result<Option<i64>, String> {
    let Some(v) = o.get(k) else {
        if required {
            return Err(format!("{k} missing"));
        }
        return Ok(None);
    };
    if v.is_null() {
        if required {
            return Err(format!("{k} missing"));
        }
        return Ok(None);
    }
    let s = v
        .as_str()
        .ok_or_else(|| format!("{k} must be a timestamp string or null"))?;
    parse_time(s).map(Some).map_err(|e| format!("{k}: {e}"))
}
fn parse_time(s: &str) -> Result<i64, String> {
    let (_, tail) = s.split_once(',').ok_or("expected RFC 1123 timestamp")?;
    let p: Vec<_> = tail.split_whitespace().collect();
    if p.len() != 5 {
        return Err("expected day month year HH:MM:SS GMT".into());
    }
    let d: u32 = p[0].parse().map_err(|_| "bad day")?;
    let m = match p[1] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return Err("bad month".into()),
    };
    let y: i64 = p[2].parse().map_err(|_| "bad year")?;
    if !matches!(p[4], "GMT" | "UTC") {
        return Err("only GMT/UTC accepted".into());
    }
    let t: Vec<_> = p[3].split(':').collect();
    if t.len() != 3 {
        return Err("bad time".into());
    }
    let h: u32 = t[0].parse().map_err(|_| "bad hour")?;
    let min: u32 = t[1].parse().map_err(|_| "bad minute")?;
    let sec: u32 = t[2].parse().map_err(|_| "bad second")?;
    let md = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    };
    if d == 0 || d > md || h > 23 || min > 59 || sec > 59 {
        return Err("date/time component out of range".into());
    }
    let ya = y - if m <= 2 { 1 } else { 0 };
    let era = if ya >= 0 { ya } else { ya - 399 } / 400;
    let yoe = ya - era * 400;
    let mp = i64::from(m) + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Ok(days * 86400 + i64::from(h) * 3600 + i64::from(min) * 60 + i64::from(sec))
}
fn parse_row(v: &Value) -> Result<Row, String> {
    let o = v.as_object().ok_or("row is not an object")?;
    let site = o
        .get("siteID")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("unknown_site")
        .to_owned();
    let c = time(o, "connectionTime", true)?.unwrap();
    let d = time(o, "disconnectTime", true)?.unwrap();
    let e = num(o, "kWhDelivered")?;
    if e < 0.0 {
        return Err("kWhDelivered must be non-negative".into());
    }
    if d < c {
        return Err("disconnectTime precedes connectionTime".into());
    }
    let done = time(o, "doneChargingTime", false)?;
    let (active, post) = if let Some(x) = done {
        if x < c {
            return Err("doneChargingTime precedes connectionTime".into());
        }
        if x > d {
            return Err("doneChargingTime follows disconnectTime".into());
        }
        (Some((x - c) as f64 / 60.0), Some((d - x) as f64 / 60.0))
    } else {
        (None, None)
    };
    Ok(Row {
        site,
        energy: e,
        dwell: (d - c) as f64 / 60.0,
        active,
        post,
    })
}
fn audit(v: &Value, m: Meta) -> Result<Report, String> {
    let rows = sessions(v)?;
    let mut all = Acc::default();
    let mut sites: BTreeMap<String, Acc> = BTreeMap::new();
    let mut bad = Vec::new();
    for (i, v) in rows.iter().enumerate() {
        match parse_row(v) {
            Ok(r) => {
                all.push(&r);
                sites.entry(r.site.clone()).or_default().push(&r)
            }
            Err(reason) => bad.push(Invalid {
                row_index_zero_based: i,
                reason,
            }),
        }
    }
    let total = rows.len();
    Ok(Report{report_schema_version:"0.1.0".into(),tool:"ev-session-audit".into(),dataset_id:m.dataset_id,source_uri:m.source_uri,evidence_class_declared_by_caller:m.evidence_class,provenance_warning:"Source identity, snapshot, evidence class and rights are caller-declared and not authenticated here. Keep source export and record SHA-256 externally.".into(),claim_ceiling:CLAIM_CEILING.into(),input_rows:total,valid_rows:total-bad.len(),invalid_rows:bad,overall:all.finish(),by_site:sites.into_iter().map(|(k,v)|(k,v.finish())).collect()})
}
fn run() -> Result<(), String> {
    let a: Vec<String> = env::args().collect();
    if a.len() < 2 {
        return Err("usage: ev-session-audit <export.json> --dataset-id ID --source-uri HTTPS_URL --evidence-class CLASS".into());
    }
    let mut id = None;
    let mut uri = None;
    let mut class = None;
    let mut i = 2;
    while i < a.len() {
        if i + 1 >= a.len() {
            return Err(format!("{} needs a value", a[i]));
        }
        match a[i].as_str() {
            "--dataset-id" => id = Some(a[i + 1].clone()),
            "--source-uri" => uri = Some(a[i + 1].clone()),
            "--evidence-class" => class = Some(a[i + 1].clone()),
            x => return Err(format!("unknown argument {x}")),
        }
        i += 2
    }
    let m = Meta {
        dataset_id: id
            .filter(|s| !s.trim().is_empty())
            .ok_or("--dataset-id required")?,
        source_uri: uri
            .filter(|s| s.starts_with("https://"))
            .ok_or("--source-uri must begin with https://")?,
        evidence_class: class
            .filter(|s| !s.trim().is_empty())
            .ok_or("--evidence-class required")?,
    };
    let bytes = fs::read(&a[1]).map_err(|e| format!("cannot read input: {e}"))?;
    let v: Value = serde_json::from_slice(&bytes).map_err(|e| format!("invalid JSON: {e}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&audit(&v, m)?).map_err(|e| e.to_string())?
    );
    Ok(())
}
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ev-session-audit: {e}");
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn meta() -> Meta {
        Meta {
            dataset_id: "fixture".into(),
            source_uri: "https://example.invalid".into(),
            evidence_class: "synthetic".into(),
        }
    }
    fn row(c: &str, d: &str, done: Value, kwh: Value) -> Value {
        serde_json::json!({"sessionID":"example-id","userID":"example-user","siteID":"site-a","connectionTime":c,"disconnectTime":d,"doneChargingTime":done,"kWhDelivered":kwh})
    }
    #[test]
    fn accounts_all_rows_and_computes_dwell_and_idle() {
        let v = serde_json::json!({"_items":[row("Wed, 1 May 2019 08:00:00 GMT","Wed, 1 May 2019 10:30:00 GMT",serde_json::json!("Wed, 1 May 2019 09:00:00 GMT"),serde_json::json!(12.5)),row("Wed, 1 May 2019 12:00:00 GMT","Wed, 1 May 2019 13:00:00 GMT",Value::Null,serde_json::json!(7.5)),row("Wed, 1 May 2019 14:00:00 GMT","Wed, 1 May 2019 13:00:00 GMT",Value::Null,serde_json::json!(5.0)),row("Wed, 1 May 2019 15:00:00 GMT","Wed, 1 May 2019 16:00:00 GMT",Value::Null,serde_json::json!(-1.0))]});
        let r = audit(&v, meta()).unwrap();
        assert_eq!(
            (r.input_rows, r.valid_rows, r.invalid_rows.len()),
            (4, 2, 2)
        );
        assert_eq!(r.overall.energy_delivered_kwh_total, 20.0);
        assert_eq!(r.overall.energy_delivered_kwh.p50, Some(10.0));
        assert_eq!(r.overall.energy_delivered_kwh.p95, Some(12.5));
        assert_eq!(r.overall.connected_dwell_minutes.p50, Some(105.0));
        assert_eq!(r.overall.active_charging_minutes.p50, Some(60.0));
        assert_eq!(r.overall.post_charge_occupancy_minutes.p50, Some(90.0));
        assert_eq!(r.overall.sessions_without_done_charging_time, 1);
    }
    #[test]
    fn supports_array_and_named_wrapper() {
        let r = row(
            "Wed, 1 May 2019 08:00:00 GMT",
            "Wed, 1 May 2019 09:00:00 GMT",
            Value::Null,
            serde_json::json!(1),
        );
        assert_eq!(sessions(&serde_json::json!([r.clone()])).unwrap().len(), 1);
        assert_eq!(
            sessions(&serde_json::json!({"sessions":[r]}))
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn rejects_malformed_and_non_utc_times() {
        assert!(parse_time("not a time").is_err());
        assert!(parse_time("Wed, 1 May 2019 08:00:00 PDT").is_err());
        assert!(parse_time("Wed, 31 Feb 2019 08:00:00 GMT").is_err());
    }
    #[test]
    fn parses_epoch_and_leap_year() {
        assert_eq!(parse_time("Thu, 1 Jan 1970 00:00:00 GMT").unwrap(), 0);
        assert_eq!(
            parse_time("Sun, 1 Mar 2020 00:00:00 GMT").unwrap()
                - parse_time("Sat, 29 Feb 2020 00:00:00 GMT").unwrap(),
            86400
        );
    }
    #[test]
    fn rejects_done_time_outside_session() {
        let v = row(
            "Wed, 1 May 2019 08:00:00 GMT",
            "Wed, 1 May 2019 10:00:00 GMT",
            serde_json::json!("Wed, 1 May 2019 11:00:00 GMT"),
            serde_json::json!(1),
        );
        assert!(parse_row(&v).unwrap_err().contains("follows"));
    }
    #[test]
    fn report_excludes_source_row_identifiers() {
        let v = serde_json::json!([row(
            "Wed, 1 May 2019 08:00:00 GMT",
            "Wed, 1 May 2019 09:00:00 GMT",
            Value::Null,
            serde_json::json!(1)
        )]);
        let s = serde_json::to_string(&audit(&v, meta()).unwrap()).unwrap();
        assert!(!s.contains("example-id"));
        assert!(!s.contains("example-user"));
    }
    #[test]
    fn missing_distribution_is_null_not_zero() {
        let d = dist(vec![]);
        assert_eq!(d.count, 0);
        assert_eq!(d.p95, None);
    }
    #[test]
    fn nearest_rank_p95_and_even_median() {
        assert_eq!(dist(vec![1., 2., 3., 4., 100.]).p95, Some(100.));
        assert_eq!(dist(vec![2., 4.]).p50, Some(3.));
    }
}
