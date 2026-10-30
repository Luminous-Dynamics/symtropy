//! Manufacturing digital-thread ledger validator for electrical equipment.
//! Validates declared component genealogy and chronological process/inspection/test events.
//! It does not decide product conformity or substitute for approved inspection/test plans.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 21] = [
    "product_family", "product_id", "serial_number", "component_id", "component_type",
    "component_revision", "component_lot_id", "parent_component_id", "event_id",
    "sequence_no", "event_type", "operation_code", "station_id", "recipe_sha256",
    "start_utc", "end_utc", "disposition", "energy_kwh", "scrap_kg",
    "evidence_artifact_id", "evidence_sha256",
];

#[derive(Clone, Debug, PartialEq, Eq)]
struct ComponentIdentity {
    component_type: String,
    revision: String,
    lot_id: String,
    parent_id: Option<String>,
}

#[derive(Clone, Debug)]
struct Event {
    product_id: String,
    serial_number: String,
    component_id: String,
    identity: ComponentIdentity,
    event_id: String,
    sequence_no: u64,
    event_type: String,
    operation_code: String,
    station_id: String,
    recipe_sha256: String,
    start: i64,
    end: i64,
    disposition: String,
    energy_kwh: f64,
    scrap_kg: f64,
    evidence_artifact_id: String,
    evidence_sha256: String,
}

#[derive(Clone, Debug, Default)]
struct ProductSummary {
    product_family: String,
    product_id: String,
    serial_number: String,
    component_count: usize,
    event_count: usize,
    process_event_count: usize,
    inspection_event_count: usize,
    test_event_count: usize,
    rework_event_count: usize,
    failed_event_count: usize,
    unknown_event_count: usize,
    total_energy_kwh: f64,
    total_scrap_kg: f64,
    final_release_pass_event_count: usize,
}

#[derive(Clone, Debug)]
struct Product {
    family: String,
    serial_number: String,
    components: BTreeMap<String, ComponentIdentity>,
    events: Vec<Event>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("manufacturing-ledger: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <manufacturing-events.csv> <product-summary.csv>",
            args.first().map(String::as_str).unwrap_or("manufacturing-ledger")
        ));
    }
    let products = read_ledger(&args[1])?;
    let summaries = validate_and_summarize(&products)?;
    write_summary(&args[2], &summaries)?;
    eprintln!(
        "manufacturing trace validation passed: products={}, events={}, components={}; route completeness and product conformity are not determined by this ledger",
        summaries.len(),
        summaries.iter().map(|row| row.event_count).sum::<usize>(),
        summaries.iter().map(|row| row.component_count).sum::<usize>()
    );
    Ok(())
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { Err(format!("{context}: empty value is not allowed")) }
    else { Ok(value.to_string()) }
}

fn finite_number(value: &str, context: &str) -> Result<f64, String> {
    let parsed: f64 = value.trim().parse()
        .map_err(|_| format!("{context}: expected finite number, got {value:?}"))?;
    if !parsed.is_finite() { return Err(format!("{context}: non-finite number")); }
    Ok(parsed)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_utc(value: &str, context: &str) -> Result<i64, String> {
    let b = value.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T'
        || b[13] != b':' || b[16] != b':' || b[19] != b'Z'
    {
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
        || !(0..=59).contains(&minute) || !(0..=59).contains(&second)
    {
        return Err(format!("{context}: invalid calendar/time"));
    }
    let mut days = 0_i64;
    for y in 1970..year { days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 }; }
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

fn read_ledger(path: &str) -> Result<BTreeMap<String, Product>, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing header"))?.trim_end_matches('\r');
    if header.split(',').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(format!("{path}: expected header [{}]", HEADER.join(",")));
    }

    let mut products: BTreeMap<String, Product> = BTreeMap::new();
    let mut event_ids = BTreeSet::new();

    for (index, raw_line) in lines.enumerate() {
        let line_no = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() { return Err(format!("{path}:{line_no}: blank rows are not allowed")); }
        if line.contains('"') { return Err(format!("{path}:{line_no}: quoted fields are not supported")); }
        let f: Vec<&str> = line.split(',').collect();
        if f.len() != HEADER.len() { return Err(format!("{path}:{line_no}: expected {} fields, got {}", HEADER.len(), f.len())); }

        let family = nonempty(f[0], "product_family")?;
        let product_id = nonempty(f[1], "product_id")?;
        let serial_number = nonempty(f[2], "serial_number")?;
        let component_id = nonempty(f[3], "component_id")?;
        let identity = ComponentIdentity {
            component_type: nonempty(f[4], "component_type")?,
            revision: nonempty(f[5], "component_revision")?,
            lot_id: nonempty(f[6], "component_lot_id")?,
            parent_id: if f[7].trim().is_empty() { None } else { Some(f[7].trim().to_string()) },
        };
        if identity.parent_id.as_deref() == Some(component_id.as_str()) {
            return Err(format!("{path}:{line_no}: component cannot be its own parent"));
        }
        let event_id = nonempty(f[8], "event_id")?;
        if !event_ids.insert(event_id.clone()) { return Err(format!("{path}:{line_no}: duplicate event_id {event_id:?}")); }
        let sequence_no: u64 = f[9].trim().parse().map_err(|_| format!("{path}:{line_no}: sequence_no must be a positive integer"))?;
        if sequence_no == 0 { return Err(format!("{path}:{line_no}: sequence_no must be positive")); }
        let event_type = nonempty(f[10], "event_type")?;
        if !matches!(event_type.as_str(), "MATERIAL_RECEIPT" | "PROCESS" | "INSPECTION" | "TEST" | "REWORK" | "NCR" | "PACKAGING" | "FINAL_RELEASE") {
            return Err(format!("{path}:{line_no}: unsupported event_type {event_type:?}"));
        }
        let operation_code = nonempty(f[11], "operation_code")?;
        let station_id = nonempty(f[12], "station_id")?;
        let recipe_sha256 = f[13].trim().to_ascii_lowercase();
        if matches!(event_type.as_str(), "PROCESS" | "REWORK") && !valid_sha256(&recipe_sha256) {
            return Err(format!("{path}:{line_no}: PROCESS/REWORK requires a 64-character recipe SHA-256"));
        }
        if !recipe_sha256.is_empty() && !valid_sha256(&recipe_sha256) {
            return Err(format!("{path}:{line_no}: recipe_sha256 must be empty or 64 hexadecimal characters"));
        }
        let start = parse_utc(f[14], &format!("{path}:{line_no}: start_utc"))?;
        let end = parse_utc(f[15], &format!("{path}:{line_no}: end_utc"))?;
        if end <= start { return Err(format!("{path}:{line_no}: end_utc must be after start_utc")); }
        let disposition = nonempty(f[16], "disposition")?;
        if !matches!(disposition.as_str(), "PASS" | "FAIL" | "REWORK" | "UNKNOWN" | "NA") {
            return Err(format!("{path}:{line_no}: disposition must be PASS/FAIL/REWORK/UNKNOWN/NA"));
        }
        let energy_kwh = finite_number(f[17], "energy_kwh")?;
        let scrap_kg = finite_number(f[18], "scrap_kg")?;
        if energy_kwh < 0.0 || scrap_kg < 0.0 {
            return Err(format!("{path}:{line_no}: energy and scrap must be non-negative"));
        }
        let evidence_artifact_id = nonempty(f[19], "evidence_artifact_id")?;
        let evidence_sha256 = nonempty(f[20], "evidence_sha256")?.to_ascii_lowercase();
        if !valid_sha256(&evidence_sha256) {
            return Err(format!("{path}:{line_no}: evidence_sha256 must be 64 hexadecimal characters"));
        }
        if matches!(event_type.as_str(), "INSPECTION" | "TEST" | "NCR" | "FINAL_RELEASE")
            && evidence_artifact_id.is_empty()
        {
            return Err(format!("{path}:{line_no}: quality events require evidence artifact"));
        }

        let product = products.entry(product_id.clone()).or_insert_with(|| Product {
            family: family.clone(),
            serial_number: serial_number.clone(),
            components: BTreeMap::new(),
            events: Vec::new(),
        });
        if product.family != family || product.serial_number != serial_number {
            return Err(format!("{path}:{line_no}: product family/serial changed for product_id {product_id:?}"));
        }
        match product.components.get(&component_id) {
            Some(existing) if existing != &identity => {
                return Err(format!("{path}:{line_no}: component identity or parent changed for {component_id:?}"));
            }
            None => { product.components.insert(component_id.clone(), identity.clone()); }
            _ => {}
        }
        product.events.push(Event {
            product_id, serial_number, component_id, identity, event_id, sequence_no,
            event_type, operation_code, station_id, recipe_sha256, start, end,
            disposition, energy_kwh, scrap_kg, evidence_artifact_id, evidence_sha256,
        });
    }
    if products.is_empty() { return Err(format!("{path}: no manufacturing events")); }
    Ok(products)
}

fn validate_genealogy(product_id: &str, product: &Product) -> Result<(), String> {
    for (component_id, identity) in &product.components {
        if let Some(parent) = &identity.parent_id {
            if !product.components.contains_key(parent) {
                return Err(format!("product {product_id:?}: component {component_id:?} references missing parent {parent:?}"));
            }
        }
        let mut visited = BTreeSet::new();
        let mut current = Some(component_id.as_str());
        while let Some(id) = current {
            if !visited.insert(id.to_string()) {
                return Err(format!("product {product_id:?}: component genealogy cycle includes {id:?}"));
            }
            current = product.components.get(id).and_then(|node| node.parent_id.as_deref());
        }
    }
    Ok(())
}

fn validate_and_summarize(products: &BTreeMap<String, Product>) -> Result<Vec<ProductSummary>, String> {
    let mut summaries = Vec::new();
    for (product_id, product) in products {
        validate_genealogy(product_id, product)?;
        let mut by_component: BTreeMap<String, Vec<&Event>> = BTreeMap::new();
        for event in &product.events {
            by_component.entry(event.component_id.clone()).or_default().push(event);
        }
        for (component_id, events) in &mut by_component {
            events.sort_by(|a, b| a.sequence_no.cmp(&b.sequence_no));
            for (index, event) in events.iter().enumerate() {
                let expected = index as u64 + 1;
                if event.sequence_no != expected {
                    return Err(format!("product {product_id:?}, component {component_id:?}: expected sequence {expected}, got {}", event.sequence_no));
                }
                if index > 0 {
                    let previous = events[index - 1];
                    if event.start < previous.end {
                        return Err(format!("product {product_id:?}, component {component_id:?}: events overlap or are out of chronological order"));
                    }
                }
            }
        }

        let mut summary = ProductSummary {
            product_family: product.family.clone(),
            product_id: product_id.clone(),
            serial_number: product.serial_number.clone(),
            component_count: product.components.len(),
            event_count: product.events.len(),
            ..ProductSummary::default()
        };
        for event in &product.events {
            summary.total_energy_kwh += event.energy_kwh;
            summary.total_scrap_kg += event.scrap_kg;
            match event.event_type.as_str() {
                "PROCESS" => summary.process_event_count += 1,
                "INSPECTION" => summary.inspection_event_count += 1,
                "TEST" => summary.test_event_count += 1,
                "REWORK" => summary.rework_event_count += 1,
                _ => {}
            }
            if event.disposition == "FAIL" { summary.failed_event_count += 1; }
            if event.disposition == "UNKNOWN" { summary.unknown_event_count += 1; }
            if event.event_type == "FINAL_RELEASE" && event.disposition == "PASS" {
                summary.final_release_pass_event_count += 1;
            }
            // Access declared event evidence and recipe fields to make clear they are part of each event.
            let _evidence_binding = (&event.event_id, &event.recipe_sha256, &event.evidence_artifact_id, &event.evidence_sha256, &event.identity, &event.operation_code, &event.station_id, &event.start, &event.end, &event.sequence_no, &event.product_id, &event.serial_number);
        }
        if !summary.total_energy_kwh.is_finite() || !summary.total_scrap_kg.is_finite() {
            return Err(format!("product {product_id:?}: accumulated energy or scrap is non-finite"));
        }
        summaries.push(summary);
    }
    Ok(summaries)
}

fn write_summary(path: &str, rows: &[ProductSummary]) -> Result<(), String> {
    let mut output = String::from("product_family,product_id,serial_number,component_count,event_count,process_event_count,inspection_event_count,test_event_count,rework_event_count,failed_event_count,unknown_event_count,total_energy_kwh,total_scrap_kg,final_release_pass_event_count\n");
    for row in rows {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{:.9},{:.9},{}\n",
            row.product_family, row.product_id, row.serial_number, row.component_count,
            row.event_count, row.process_event_count, row.inspection_event_count,
            row.test_event_count, row.rework_event_count, row.failed_event_count,
            row.unknown_event_count, row.total_energy_kwh, row.total_scrap_kg,
            row.final_release_pass_event_count
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(component_type: &str, parent: Option<&str>) -> ComponentIdentity {
        ComponentIdentity {
            component_type: component_type.to_string(),
            revision: "A".to_string(),
            lot_id: "LOT-1".to_string(),
            parent_id: parent.map(str::to_string),
        }
    }

    fn event(component: &str, seq: u64, event_id: &str, start: i64, end: i64) -> Event {
        Event {
            product_id: "P1".to_string(), serial_number: "S1".to_string(),
            component_id: component.to_string(), identity: identity("WINDING", None),
            event_id: event_id.to_string(), sequence_no: seq,
            event_type: "PROCESS".to_string(), operation_code: "WIND".to_string(),
            station_id: "ST-1".to_string(), recipe_sha256: "a".repeat(64),
            start, end, disposition: "PASS".to_string(), energy_kwh: 2.0, scrap_kg: 0.1,
            evidence_artifact_id: format!("EV-{event_id}"), evidence_sha256: "b".repeat(64),
        }
    }

    #[test]
    fn utc_parser_validates_real_calendar_timestamps() {
        assert!(parse_utc("2024-02-29T00:00:00Z", "test").is_ok());
        assert!(parse_utc("2025-02-29T00:00:00Z", "test").is_err());
        assert!(parse_utc("2025-01-01T24:00:00Z", "test").is_err());
        assert!(parse_utc("2025-01-01T00:00:00+02:00", "test").is_err());
    }

    #[test]
    fn component_events_must_have_contiguous_sequence_and_nonoverlap() {
        let mut product = Product {
            family: "POWER_TRANSFORMER".to_string(), serial_number: "S1".to_string(),
            components: BTreeMap::from([("C1".to_string(), identity("WINDING", None))]),
            events: vec![event("C1", 1, "e1", 10, 20), event("C1", 2, "e2", 20, 30)],
        };
        let map = BTreeMap::from([("P1".to_string(), product.clone())]);
        assert!(validate_and_summarize(&map).is_ok());
        product.events[1].sequence_no = 3;
        let map = BTreeMap::from([("P1".to_string(), product)]);
        assert!(validate_and_summarize(&map).unwrap_err().contains("expected sequence 2"));
    }

    #[test]
    fn genealogy_rejects_missing_parent_and_cycles() {
        let product = Product {
            family: "POWER_TRANSFORMER".to_string(), serial_number: "S1".to_string(),
            components: BTreeMap::from([("C1".to_string(), identity("WINDING", Some("MISSING")))]),
            events: Vec::new(),
        };
        assert!(validate_genealogy("P1", &product).unwrap_err().contains("missing parent"));
        let cycle = Product {
            family: "POWER_TRANSFORMER".to_string(), serial_number: "S1".to_string(),
            components: BTreeMap::from([
                ("A".to_string(), identity("CORE", Some("B"))),
                ("B".to_string(), identity("TANK", Some("A"))),
            ]),
            events: Vec::new(),
        };
        assert!(validate_genealogy("P1", &cycle).unwrap_err().contains("cycle"));
    }

    #[test]
    fn checked_transformer_fixture_preserves_component_genealogy_and_quality_events() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/manufacturing-ledger-synthetic.csv");
        let products = read_ledger(path.to_str().unwrap()).unwrap();
        let summaries = validate_and_summarize(&products).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].component_count, 4);
        assert_eq!(summaries[0].event_count, 16);
        assert_eq!(summaries[0].failed_event_count, 1);
        assert_eq!(summaries[0].rework_event_count, 1);
        assert_eq!(summaries[0].test_event_count, 4);
        assert_eq!(summaries[0].final_release_pass_event_count, 1);
    }

    #[test]
    fn valid_sha256_is_strict() {
        assert!(valid_sha256(&"a".repeat(64)));
        assert!(!valid_sha256("sha256"));
        assert!(!valid_sha256(&"g".repeat(64)));
    }

    #[test]
    fn rejects_overlapping_events_for_same_component() {
        let product = Product {
            family: "POWER_TRANSFORMER".to_string(), serial_number: "S1".to_string(),
            components: BTreeMap::from([("C1".to_string(), identity("WINDING", None))]),
            events: vec![event("C1", 1, "e1", 10, 25), event("C1", 2, "e2", 20, 30)],
        };
        let map = BTreeMap::from([("P1".to_string(), product)]);
        assert!(validate_and_summarize(&map).unwrap_err().contains("overlap"));
    }
}
