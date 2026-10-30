//! Evidence-aware manufacturing route and measurement acceptance screen.
//! This is a decision-support validator, not a product release authority.
//! It checks approved route coverage, test-plan measurements, calibration validity,
//! uncertainty guard bands, and unresolved fail/rework/test sequences.
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const EVENT_HEADER: [&str; 21] = [
    "product_family", "product_id", "serial_number", "component_id", "component_type",
    "component_revision", "component_lot_id", "parent_component_id", "event_id",
    "sequence_no", "event_type", "operation_code", "station_id", "recipe_sha256",
    "start_utc", "end_utc", "disposition", "energy_kwh", "scrap_kg",
    "evidence_artifact_id", "evidence_sha256",
];
const ASSIGNMENT_HEADER: [&str; 5] = [
    "product_id", "product_family", "serial_number", "route_revision", "route_manifest_sha256",
];
const ROUTE_HEADER: [&str; 8] = [
    "product_family", "route_revision", "step_no", "operation_code",
    "component_type", "event_type", "required", "required_disposition",
];
const TEST_HEADER: [&str; 12] = [
    "product_family", "route_revision", "test_id", "component_type", "measurand",
    "unit", "lower_limit", "upper_limit", "coverage_factor",
    "max_standard_uncertainty", "calibration_class", "required",
];
const CALIBRATION_HEADER: [&str; 8] = [
    "calibration_id", "instrument_id", "calibration_class", "valid_from_utc",
    "valid_to_utc", "status", "certificate_artifact_id", "certificate_sha256",
];
const MEASUREMENT_HEADER: [&str; 13] = [
    "measurement_id", "product_id", "component_id", "test_id", "measurand",
    "observed_value", "unit", "standard_uncertainty", "instrument_id",
    "calibration_id", "measured_utc", "evidence_artifact_id", "evidence_sha256",
];

#[derive(Clone, Debug)]
struct Event {
    family: String, product: String, serial: String, component: String, component_type: String,
    event_id: String, event_type: String, operation: String, disposition: String,
    start: i64, end: i64,
}
#[derive(Clone, Debug)]
struct Assignment {
    family: String, serial: String, revision: String,
}
#[derive(Clone, Debug)]
struct RouteStep {
    family: String, revision: String, step: u64, operation: String,
    component_type: String, event_type: String, required: bool, disposition: String,
}
#[derive(Clone, Debug)]
struct TestSpec {
    family: String, revision: String, test_id: String, component_type: String,
    measurand: String, unit: String, lower: f64, upper: f64,
    coverage: f64, max_uncertainty: f64, calibration_class: String, required: bool,
}
#[derive(Clone, Debug)]
struct Calibration {
    instrument: String, class: String, valid_from: String, valid_to: String,
    status: String, certificate_id: String,
}
#[derive(Clone, Debug)]
struct Measurement {
    id: String, product: String, component: String, test_id: String, measurand: String,
    value: f64, unit: String, uncertainty: f64, instrument: String,
    calibration_id: String, measured_at: String,
}
#[derive(Clone, Debug, Default)]
struct ProductReport {
    product: String, serial: String, revision: String,
    missing_route_steps: usize, route_disposition_failures: usize,
    required_measurements: usize, passing: usize, failed: usize,
    indeterminate: usize, invalid: usize, missing_measurements: usize,
    unresolved_test_failures: usize, blockers: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("manufacturing-acceptance: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 8 {
        return Err(format!(
            "usage: {} <events.csv> <product-routes.csv> <route-plan.csv> <test-plan.csv> <calibrations.csv> <measurements.csv> <report.csv>",
            args.first().map(String::as_str).unwrap_or("manufacturing-acceptance")
        ));
    }
    let events = read_events(&args[1])?;
    let assignments = read_assignments(&args[2])?;
    let routes = read_routes(&args[3])?;
    let tests = read_test_specs(&args[4])?;
    let calibrations = read_calibrations(&args[5])?;
    let measurements = read_measurements(&args[6])?;
    let reports = evaluate(&events, &assignments, &routes, &tests, &calibrations, &measurements)?;
    write_report(&args[7], &reports)?;
    let eligible = reports.iter().filter(|r| r.blockers.is_empty()).count();
    eprintln!(
        "manufacturing acceptance screen complete: products={}, eligible_for_authorized_review={}, blocked={}; no product release was authorized",
        reports.len(), eligible, reports.len() - eligible
    );
    Ok(())
}

fn read_table(path: &str, expected: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing CSV header"))?.trim_end_matches('\r');
    if header != expected.join(",") {
        return Err(format!("{path}: unexpected CSV header; expected {}", expected.join(",")));
    }
    let mut rows = Vec::new();
    for (i, raw) in lines.enumerate() {
        let line_no = i + 2;
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() { return Err(format!("{path}:{line_no}: blank row")); }
        if line.contains('"') { return Err(format!("{path}:{line_no}: quoted CSV fields are unsupported")); }
        let fields: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
        if fields.len() != expected.len() {
            return Err(format!("{path}:{line_no}: expected {} fields, got {}", expected.len(), fields.len()));
        }
        rows.push(fields);
    }
    if rows.is_empty() { return Err(format!("{path}: no data rows")); }
    Ok(rows)
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { Err(format!("{context}: empty value")) } else { Ok(value.to_string()) }
}
fn finite(value: &str, context: &str) -> Result<f64, String> {
    let parsed: f64 = value.parse().map_err(|_| format!("{context}: expected finite number, got {value:?}"))?;
    if !parsed.is_finite() { return Err(format!("{context}: non-finite number")); }
    Ok(parsed)
}
fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn required_bool(value: &str, context: &str) -> Result<bool, String> {
    match value { "true" => Ok(true), "false" => Ok(false), _ => Err(format!("{context}: expected true or false")) }
}

// Strict canonical UTC parser. Its integer result allows exact chronology checks without a date crate.
fn parse_utc(value: &str, context: &str) -> Result<i64, String> {
    let b = value.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T'
        || b[13] != b':' || b[16] != b':' || b[19] != b'Z' {
        return Err(format!("{context}: timestamp must be YYYY-MM-DDTHH:MM:SSZ"));
    }
    let part = |range: std::ops::Range<usize>| -> Result<i32, String> {
        if !b[range.clone()].iter().all(u8::is_ascii_digit) {
            return Err(format!("{context}: non-numeric timestamp field"));
        }
        std::str::from_utf8(&b[range]).map_err(|_| format!("{context}: invalid timestamp"))?
            .parse().map_err(|_| format!("{context}: invalid timestamp number"))
    };
    let year = part(0..4)?;
    let month = part(5..7)?;
    let day = part(8..10)?;
    let hour = part(11..13)?;
    let minute = part(14..16)?;
    let second = part(17..19)?;
    if !(1900..=2200).contains(&year) || !(1..=12).contains(&month) {
        return Err(format!("{context}: year/month out of range"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => unreachable!(),
    };
    if !(1..=month_days).contains(&day) || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return Err(format!("{context}: invalid calendar/time"));
    }
    let mut days = 0_i64;
    if year >= 1970 {
        for y in 1970..year { days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 }; }
    } else {
        for y in year..1970 { days -= if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 }; }
    }
    for m in 1..month {
        days += match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => unreachable!(),
        };
    }
    days += i64::from(day - 1);
    Ok(days * 86_400 + i64::from(hour * 3_600 + minute * 60 + second))
}

fn read_events(path: &str) -> Result<Vec<Event>, String> {
    let rows = read_table(path, &EVENT_HEADER)?;
    let mut events = Vec::new();
    let mut ids = BTreeSet::new();
    for (i, f) in rows.iter().enumerate() {
        let row = i + 2;
        let event_id = nonempty(&f[8], "event_id")?;
        if !ids.insert(event_id.clone()) { return Err(format!("{path}:{row}: duplicate event_id {event_id:?}")); }
        let start = parse_utc(&f[14], "event start")?;
        let end = parse_utc(&f[15], "event end")?;
        if end <= start { return Err(format!("{path}:{row}: event end must follow start")); }
        events.push(Event {
            family: nonempty(&f[0], "product_family")?,
            product: nonempty(&f[1], "product_id")?,
            serial: nonempty(&f[2], "serial_number")?,
            component: nonempty(&f[3], "component_id")?,
            component_type: nonempty(&f[4], "component_type")?,
            event_id, event_type: nonempty(&f[10], "event_type")?,
            operation: nonempty(&f[11], "operation_code")?,
            disposition: nonempty(&f[16], "disposition")?,
            start, end,
        });
    }
    Ok(events)
}

fn read_assignments(path: &str) -> Result<BTreeMap<String, Assignment>, String> {
    let rows = read_table(path, &ASSIGNMENT_HEADER)?;
    let mut output = BTreeMap::new();
    for f in rows {
        let product = nonempty(&f[0], "product_id")?;
        let digest = nonempty(&f[4], "route_manifest_sha256")?;
        if !valid_sha256(&digest) { return Err(format!("{path}: invalid route manifest SHA-256 for {product:?}")); }
        let value = Assignment {
            family: nonempty(&f[1], "product_family")?,
            serial: nonempty(&f[2], "serial_number")?,
            revision: nonempty(&f[3], "route_revision")?,
        };
        if output.insert(product.clone(), value).is_some() {
            return Err(format!("{path}: duplicate product assignment {product:?}"));
        }
    }
    Ok(output)
}

fn read_routes(path: &str) -> Result<Vec<RouteStep>, String> {
    let rows = read_table(path, &ROUTE_HEADER)?;
    let mut output = Vec::new();
    let mut keys = BTreeSet::new();
    for f in rows {
        let family = nonempty(&f[0], "route product_family")?;
        let revision = nonempty(&f[1], "route_revision")?;
        let step: u64 = f[2].parse().map_err(|_| format!("{path}: invalid step_no"))?;
        if step == 0 { return Err(format!("{path}: step_no must be positive")); }
        let operation = nonempty(&f[3], "operation_code")?;
        let component_type = nonempty(&f[4], "component_type")?;
        let event_type = nonempty(&f[5], "event_type")?;
        if !matches!(event_type.as_str(), "MATERIAL_RECEIPT" | "PROCESS" | "INSPECTION" | "TEST" | "REWORK" | "NCR" | "PACKAGING" | "FINAL_RELEASE") {
            return Err(format!("{path}: unsupported event_type {event_type:?}"));
        }
        let required = required_bool(&f[6], "route required")?;
        let disposition = nonempty(&f[7], "required_disposition")?;
        let key = (family.clone(), revision.clone(), step);
        if !keys.insert(key) { return Err(format!("{path}: duplicate route step {step} for {family}/{revision}")); }
        output.push(RouteStep { family, revision, step, operation, component_type, event_type, required, disposition });
    }
    Ok(output)
}

fn read_test_specs(path: &str) -> Result<Vec<TestSpec>, String> {
    let rows = read_table(path, &TEST_HEADER)?;
    let mut output = Vec::new();
    let mut keys = BTreeSet::new();
    for f in rows {
        let family = nonempty(&f[0], "test product_family")?;
        let revision = nonempty(&f[1], "test route_revision")?;
        let test_id = nonempty(&f[2], "test_id")?;
        let component_type = nonempty(&f[3], "test component_type")?;
        let measurand = nonempty(&f[4], "measurand")?;
        let unit = nonempty(&f[5], "unit")?;
        let lower = finite(&f[6], "lower_limit")?;
        let upper = finite(&f[7], "upper_limit")?;
        let coverage = finite(&f[8], "coverage_factor")?;
        let max_uncertainty = finite(&f[9], "max_standard_uncertainty")?;
        if lower >= upper || coverage <= 0.0 || max_uncertainty <= 0.0 {
            return Err(format!("{path}: invalid limits, coverage factor or uncertainty ceiling for {test_id:?}"));
        }
        let calibration_class = nonempty(&f[10], "calibration_class")?;
        let required = required_bool(&f[11], "test required")?;
        let key = (family.clone(), revision.clone(), test_id.clone(), component_type.clone(), measurand.clone());
        if !keys.insert(key) { return Err(format!("{path}: duplicate test specification {test_id:?}")); }
        output.push(TestSpec { family, revision, test_id, component_type, measurand, unit, lower, upper, coverage, max_uncertainty, calibration_class, required });
    }
    Ok(output)
}

fn read_calibrations(path: &str) -> Result<BTreeMap<String, Calibration>, String> {
    let rows = read_table(path, &CALIBRATION_HEADER)?;
    let mut output = BTreeMap::new();
    for f in rows {
        let id = nonempty(&f[0], "calibration_id")?;
        let from = nonempty(&f[3], "calibration valid_from")?;
        let to = nonempty(&f[4], "calibration valid_to")?;
        let from_epoch = parse_utc(&from, "calibration valid_from")?;
        let to_epoch = parse_utc(&to, "calibration valid_to")?;
        if to_epoch <= from_epoch { return Err(format!("{path}: calibration validity interval is empty or reversed")); }
        let status = nonempty(&f[5], "calibration status")?;
        if !matches!(status.as_str(), "CURRENT" | "EXPIRED" | "SUSPENDED" | "REVOKED") {
            return Err(format!("{path}: unknown calibration status {status:?}"));
        }
        let certificate_id = nonempty(&f[6], "calibration certificate artifact id")?;
        if !valid_sha256(&f[7]) { return Err(format!("{path}: malformed certificate SHA-256 for {id:?}")); }
        let value = Calibration {
            instrument: nonempty(&f[1], "instrument_id")?,
            class: nonempty(&f[2], "calibration_class")?,
            valid_from: from, valid_to: to, status, certificate_id,
        };
        if output.insert(id.clone(), value).is_some() { return Err(format!("{path}: duplicate calibration_id {id:?}")); }
    }
    Ok(output)
}

fn read_measurements(path: &str) -> Result<Vec<Measurement>, String> {
    let rows = read_table(path, &MEASUREMENT_HEADER)?;
    let mut output = Vec::new();
    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    for f in rows {
        let id = nonempty(&f[0], "measurement_id")?;
        if !ids.insert(id.clone()) { return Err(format!("{path}: duplicate measurement_id {id:?}")); }
        let product = nonempty(&f[1], "measurement product_id")?;
        let component = nonempty(&f[2], "measurement component_id")?;
        let test_id = nonempty(&f[3], "measurement test_id")?;
        let measurand = nonempty(&f[4], "measurement measurand")?;
        let unit = nonempty(&f[6], "measurement unit")?;
        let value = finite(&f[5], "observed_value")?;
        let uncertainty = finite(&f[7], "standard_uncertainty")?;
        if uncertainty < 0.0 { return Err(format!("{path}: standard_uncertainty cannot be negative")); }
        let measured_at = nonempty(&f[10], "measured_utc")?;
        parse_utc(&measured_at, "measurement timestamp")?;
        let evidence_id = nonempty(&f[11], "measurement evidence artifact id")?;
        if !valid_sha256(&f[12]) { return Err(format!("{path}: malformed evidence SHA-256 for {id:?}")); }
        let key = (product.clone(), component.clone(), test_id.clone(), measurand.clone());
        if !keys.insert(key) { return Err(format!("{path}: duplicate product/component/test/measurand measurement")); }
        output.push(Measurement {
            id, product, component, test_id, measurand, value, unit, uncertainty,
            instrument: nonempty(&f[8], "instrument_id")?,
            calibration_id: nonempty(&f[9], "calibration_id")?, measured_at,
        });
        let _ = evidence_id;
    }
    Ok(output)
}

fn classify_guardband(value: f64, uncertainty: f64, coverage: f64, lower: f64, upper: f64) -> &'static str {
    let measurement_lower = value - coverage * uncertainty;
    let measurement_upper = value + coverage * uncertainty;
    if measurement_lower >= lower && measurement_upper <= upper {
        "PASS"
    } else if measurement_upper < lower || measurement_lower > upper {
        "FAIL"
    } else {
        "INDETERMINATE"
    }
}

fn failure_resolved(failed: &Event, events: &[Event]) -> bool {
    let rework_end = events.iter()
        .filter(|e| e.product == failed.product && e.component == failed.component
            && e.event_type == "REWORK" && e.start >= failed.end)
        .map(|e| e.end).min();
    let Some(rework_end) = rework_end else { return false; };
    events.iter().any(|e| e.product == failed.product && e.component == failed.component
        && e.event_type == "TEST" && e.operation == failed.operation
        && e.disposition == "PASS" && e.start >= rework_end)
}

fn calibration_eligible(cal: &Calibration, measurement: &Measurement, spec: &TestSpec) -> bool {
    cal.status == "CURRENT"
        && cal.instrument == measurement.instrument
        && cal.class == spec.calibration_class
        && measurement.measured_at >= cal.valid_from
        && measurement.measured_at <= cal.valid_to
}

fn evaluate(
    events: &[Event],
    assignments: &BTreeMap<String, Assignment>,
    routes: &[RouteStep],
    specs: &[TestSpec],
    calibrations: &BTreeMap<String, Calibration>,
    measurements: &[Measurement],
) -> Result<Vec<ProductReport>, String> {
    let mut products: BTreeMap<String, ProductReport> = BTreeMap::new();
    for (product, assignment) in assignments {
        let matching: Vec<&Event> = events.iter().filter(|e| &e.product == product).collect();
        if matching.is_empty() { return Err(format!("product route {product:?} has no manufacturing events")); }
        if matching.iter().any(|e| e.family != assignment.family || e.serial != assignment.serial) {
            return Err(format!("product route {product:?} conflicts with event product-family/serial identity"));
        }
        products.insert(product.clone(), ProductReport {
            product: product.clone(), serial: assignment.serial.clone(), revision: assignment.revision.clone(),
            ..ProductReport::default()
        });
    }
    for event in events {
        if !assignments.contains_key(&event.product) {
            return Err(format!("manufacturing event {:?} has no approved product-route assignment", event.event_id));
        }
    }

    for (product, assignment) in assignments {
        let product_events: Vec<&Event> = events.iter().filter(|e| &e.product == product).collect();
        let route_rows: Vec<&RouteStep> = routes.iter()
            .filter(|r| r.family == assignment.family && r.revision == assignment.revision && r.required)
            .collect();
        if route_rows.is_empty() { return Err(format!("no required route steps for {product:?} {}/{}", assignment.family, assignment.revision)); }
        let mut ordered_steps = route_rows.clone();
        ordered_steps.sort_by_key(|r| r.step);
        for route in ordered_steps {
            let matched: Vec<&Event> = product_events.iter().copied().filter(|e|
                e.operation == route.operation && e.component_type == route.component_type
                && e.event_type == route.event_type).collect();
            let report = products.get_mut(product).expect("assignment report exists");
            if matched.is_empty() {
                report.missing_route_steps += 1;
            } else if !matched.iter().any(|e| e.disposition == route.disposition) {
                report.route_disposition_failures += 1;
            }
        }
        let failures: Vec<&Event> = product_events.iter().copied()
            .filter(|e| e.event_type == "TEST" && e.disposition == "FAIL").collect();
        products.get_mut(product).unwrap().unresolved_test_failures =
            failures.iter().filter(|failed| !failure_resolved(failed, events)).count();
    }

    for measurement in measurements {
        let assignment = assignments.get(&measurement.product)
            .ok_or_else(|| format!("measurement {:?} references an unassigned product", measurement.id))?;
        let spec_exists = specs.iter().any(|s| s.family == assignment.family
            && s.revision == assignment.revision && s.test_id == measurement.test_id
            && s.measurand == measurement.measurand);
        if !spec_exists { return Err(format!("measurement {:?} has no matching approved test specification", measurement.id)); }
    }

    for (product, assignment) in assignments {
        let product_events: Vec<&Event> = events.iter().filter(|e| &e.product == product).collect();
        let product_specs: Vec<&TestSpec> = specs.iter()
            .filter(|s| s.family == assignment.family && s.revision == assignment.revision && s.required)
            .collect();
        for spec in product_specs {
            let component_ids: BTreeSet<String> = product_events.iter()
                .filter(|e| e.component_type == spec.component_type)
                .map(|e| e.component.clone()).collect();
            let report = products.get_mut(product).unwrap();
            if component_ids.is_empty() {
                report.missing_measurements += 1;
                continue;
            }
            for component in component_ids {
                report.required_measurements += 1;
                let measurement = measurements.iter().find(|m| m.product == *product
                    && m.component == component && m.test_id == spec.test_id
                    && m.measurand == spec.measurand);
                let Some(measurement) = measurement else {
                    report.missing_measurements += 1;
                    continue;
                };
                let measured_epoch = parse_utc(&measurement.measured_at, "measurement timestamp")?;
                let event_backed = product_events.iter().any(|event|
                    event.component == component
                    && event.operation == spec.test_id
                    && (event.event_type == "TEST" || event.event_type == "INSPECTION")
                    && event.disposition == "PASS"
                    && measured_epoch >= event.start && measured_epoch <= event.end
                );
                if !event_backed {
                    report.invalid += 1;
                    continue;
                }
                if measurement.unit != spec.unit || measurement.uncertainty > spec.max_uncertainty {
                    report.invalid += 1;
                    continue;
                }
                let Some(calibration) = calibrations.get(&measurement.calibration_id) else {
                    report.invalid += 1;
                    continue;
                };
                if !calibration_eligible(calibration, measurement, spec) {
                    report.invalid += 1;
                    continue;
                }
                let _certificate_artifact = &calibration.certificate_id;
                match classify_guardband(measurement.value, measurement.uncertainty,
                    spec.coverage, spec.lower, spec.upper) {
                    "PASS" => report.passing += 1,
                    "FAIL" => report.failed += 1,
                    _ => report.indeterminate += 1,
                }
            }
        }
        let report = products.get_mut(product).unwrap();
        if report.missing_route_steps > 0 || report.route_disposition_failures > 0 {
            report.blockers.push("ROUTE_INCOMPLETE".to_string());
        }
        if report.missing_measurements > 0 || report.invalid > 0
            || report.failed > 0 || report.indeterminate > 0
            || report.passing != report.required_measurements {
            report.blockers.push("REQUIRED_MEASUREMENTS_NOT_PASSING".to_string());
        }
        if report.unresolved_test_failures > 0 {
            report.blockers.push("UNRESOLVED_TEST_FAILURES".to_string());
        }
    }
    Ok(products.into_values().collect())
}

fn write_report(path: &str, reports: &[ProductReport]) -> Result<(), String> {
    let mut output = String::from(
        "product_id,serial_number,route_revision,route_complete,missing_route_steps,route_disposition_failures,required_measurements,passing_measurements,failed_measurements,indeterminate_measurements,invalid_measurements,missing_measurements,unresolved_test_failures,eligible_for_authorized_review,blockers\n"
    );
    for r in reports {
        let route_complete = r.missing_route_steps == 0 && r.route_disposition_failures == 0;
        let eligible = r.blockers.is_empty();
        let blocker_text = if eligible { "NONE".to_string() } else { r.blockers.join(";") };
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.product, r.serial, r.revision, route_complete, r.missing_route_steps,
            r.route_disposition_failures, r.required_measurements, r.passing, r.failed,
            r.indeterminate, r.invalid, r.missing_measurements, r.unresolved_test_failures,
            eligible, blocker_text
        ));
    }
    fs::write(path, output).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardband_requires_the_entire_uncertainty_interval_inside_limits() {
        assert_eq!(classify_guardband(5.0, 0.1, 2.0, 4.0, 6.0), "PASS");
        assert_eq!(classify_guardband(6.1, 0.1, 2.0, 4.0, 6.0), "FAIL");
        assert_eq!(classify_guardband(5.9, 0.1, 2.0, 4.0, 6.0), "INDETERMINATE");
    }

    #[test]
    fn calibration_requires_current_status_matching_instrument_class_and_time() {
        let cal = Calibration {
            instrument: "METER-1".into(), class: "OHMMETER".into(),
            valid_from: "2026-01-01T00:00:00Z".into(),
            valid_to: "2026-12-31T23:59:59Z".into(),
            status: "CURRENT".into(), certificate_id: "CERT-1".into(),
        };
        let spec = TestSpec {
            family: "POWER_TRANSFORMER".into(), revision: "ROUTE-A".into(),
            test_id: "WINDING_RESISTANCE".into(), component_type: "HV_WINDING".into(),
            measurand: "resistance".into(), unit: "ohm".into(), lower: 0.0, upper: 1.0,
            coverage: 2.0, max_uncertainty: 0.01, calibration_class: "OHMMETER".into(), required: true,
        };
        let measurement = Measurement {
            id: "M-1".into(), product: "P-1".into(), component: "C-1".into(),
            test_id: "WINDING_RESISTANCE".into(), measurand: "resistance".into(),
            value: 0.5, unit: "ohm".into(), uncertainty: 0.001, instrument: "METER-1".into(),
            calibration_id: "CAL-1".into(), measured_at: "2026-06-01T12:00:00Z".into(),
        };
        assert!(calibration_eligible(&cal, &measurement, &spec));
        let mut expired = cal.clone();
        expired.status = "EXPIRED".into();
        assert!(!calibration_eligible(&expired, &measurement, &spec));
        let mut stale = measurement.clone();
        stale.measured_at = "2027-01-01T00:00:00Z".into();
        assert!(!calibration_eligible(&cal, &stale, &spec));
    }

    #[test]
    fn failed_test_requires_rework_then_a_later_passing_repeat() {
        let failed = Event {
            family: "F".into(), product: "P".into(), serial: "S".into(),
            component: "C".into(), component_type: "WINDING".into(),
            event_id: "FAIL".into(), event_type: "TEST".into(), operation: "RESISTANCE".into(),
            disposition: "FAIL".into(), start: 10, end: 20,
        };
        let mut events = vec![failed.clone()];
        assert!(!failure_resolved(&failed, &events));
        events.push(Event {
            event_type: "REWORK".into(), event_id: "RW".into(), operation: "CONNECTION_FIX".into(),
            disposition: "PASS".into(), start: 21, end: 30, ..failed.clone()
        });
        assert!(!failure_resolved(&failed, &events));
        events.push(Event {
            event_type: "TEST".into(), event_id: "PASS".into(), operation: "RESISTANCE".into(),
            disposition: "PASS".into(), start: 31, end: 35, ..failed.clone()
        });
        assert!(failure_resolved(&failed, &events));
    }

    #[test]
    fn sha256_syntax_and_utc_calendar_are_strict() {
        assert!(valid_sha256(&"a".repeat(64)));
        assert!(!valid_sha256("abc"));
        assert!(parse_utc("2024-02-29T23:59:59Z", "test").is_ok());
        assert!(parse_utc("2025-02-29T23:59:59Z", "test").is_err());
        assert!(parse_utc("2026-01-01T00:00:00+02:00", "test").is_err());
    }
}
