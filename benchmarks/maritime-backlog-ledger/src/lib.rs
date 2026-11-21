use std::collections::{BTreeMap, BTreeSet};

pub const CASES_CSV: &str = include_str!("../data/cases.csv");
pub const EVENTS_CSV: &str = include_str!("../data/events.csv");

const CASES_HEADER: &str = "case_id,ledger_kind,stock_unit,terminal_id,window_start_tick,as_of_tick,opening_stock,reported_closing_stock,expected_outcome";
const EVENTS_HEADER: &str = "case_id,event_id,source_record_id,source_id,revision,observed_tick,event_tick,classification,state,event_kind,quantity_unit,quantity,terminal_id";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LedgerKind {
    VesselQueue,
    ContainerStock,
}

impl LedgerKind {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "VESSEL_QUEUE" => Ok(Self::VesselQueue),
            "CONTAINER_STOCK" => Ok(Self::ContainerStock),
            _ => Err(format!("unknown ledger kind {value:?}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseDefinition {
    pub case_id: String,
    pub ledger: LedgerKind,
    pub terminal_id: String,
    pub window_start_tick: i64,
    pub as_of_tick: i64,
    pub opening_stock: u64,
    pub reported_closing_stock: u64,
    pub expected_accept: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Classification {
    Actual,
    Estimated,
    Planned,
    Requested,
}

impl Classification {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "ACTUAL" => Ok(Self::Actual),
            "ESTIMATED" => Ok(Self::Estimated),
            "PLANNED" => Ok(Self::Planned),
            "REQUESTED" => Ok(Self::Requested),
            _ => Err(format!("unknown event classification {value:?}")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecordState {
    Active,
    Voided,
}

impl RecordState {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "ACTIVE" => Ok(Self::Active),
            "VOIDED" => Ok(Self::Voided),
            _ => Err(format!("unknown record state {value:?}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEvent {
    pub case_id: String,
    pub event_id: String,
    pub source_record_id: String,
    pub source_id: String,
    pub revision: u32,
    pub observed_tick: i64,
    pub event_tick: i64,
    classification: Classification,
    state: RecordState,
    pub event_kind: String,
    pub quantity: u64,
    pub terminal_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaseResult {
    pub case_id: String,
    pub expected: String,
    pub actual: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorpusReport {
    pub accepted: usize,
    pub rejected: usize,
    pub matched: usize,
    pub total: usize,
    pub cases: Vec<CaseResult>,
}

impl CorpusReport {
    pub fn all_expected_dispositions_match(&self) -> bool {
        self.matched == self.total
    }
}

fn parse_i64(value: &str, name: &str, line: usize) -> Result<i64, String> {
    value
        .parse::<i64>()
        .map_err(|_| format!("line {line}: {name} must be an integer, got {value:?}"))
}

fn parse_u64(value: &str, name: &str, line: usize) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("line {line}: {name} must be a non-negative integer, got {value:?}"))
}

fn parse_u32(value: &str, name: &str, line: usize) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("line {line}: {name} must be a positive integer, got {value:?}"))
}

fn split_row<'a>(line: &'a str, expected_fields: usize, line_number: usize) -> Result<Vec<&'a str>, String> {
    let fields: Vec<&str> = line.split(',').map(str::trim).collect();
    if fields.len() != expected_fields {
        return Err(format!(
            "line {line_number}: expected {expected_fields} comma-separated fields, found {}",
            fields.len()
        ));
    }
    if fields.iter().any(|field| field.is_empty()) {
        return Err(format!("line {line_number}: blank fields are not permitted"));
    }
    Ok(fields)
}

/// Parse the frozen v0.1 case CSV. This deliberately is not a general CSV
/// parser: the admitted fixture has a fixed header and no quoted fields.
pub fn parse_cases(csv: &str) -> Result<Vec<CaseDefinition>, String> {
    let mut lines = csv.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().ok_or_else(|| "case CSV is empty".to_string())?.trim();
    if header != CASES_HEADER {
        return Err(format!("unexpected case CSV header: {header:?}"));
    }

    let mut seen = BTreeSet::new();
    let mut cases = Vec::new();

    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let f = split_row(line, 9, line_number)?;
        let case_id = f[0].to_string();
        if !seen.insert(case_id.clone()) {
            return Err(format!("line {line_number}: duplicate case ID {case_id:?}"));
        }

        let ledger = LedgerKind::parse(f[1])?;
        let window_start_tick = parse_i64(f[3], "window_start_tick", line_number)?;
        let as_of_tick = parse_i64(f[4], "as_of_tick", line_number)?;
        let opening_stock = parse_u64(f[5], "opening_stock", line_number)?;
        let reported_closing_stock = parse_u64(f[6], "reported_closing_stock", line_number)?;
        let expected_accept = match f[7] {
            "ACCEPT" => true,
            "REJECT" => false,
            other => return Err(format!("line {line_number}: unknown expected outcome {other:?}")),
        };

        if window_start_tick > as_of_tick {
            return Err(format!("line {line_number}: window starts after the as-of cutoff"));
        }

        cases.push(CaseDefinition {
            case_id,
            ledger,
            terminal_id: f[2].to_string(),
            window_start_tick,
            as_of_tick,
            opening_stock,
            reported_closing_stock,
            expected_accept,
        });
    }

    if cases.is_empty() {
        return Err("case CSV contains no cases".to_string());
    }
    Ok(cases)
}

/// Parse source observations. Exact duplicate source_record_id rows are kept
/// here and made idempotent by the ledger verifier; conflicting reuse rejects.
pub fn parse_events(csv: &str) -> Result<Vec<SourceEvent>, String> {
    let mut lines = csv.lines().filter(|line| !line.trim().is_empty());
    let header = lines.next().ok_or_else(|| "event CSV is empty".to_string())?.trim();
    if header != EVENTS_HEADER {
        return Err(format!("unexpected event CSV header: {header:?}"));
    }

    let mut events = Vec::new();
    for (index, line) in lines.enumerate() {
        let line_number = index + 2;
        let f = split_row(line, 13, line_number)?;
        let event = SourceEvent {
            case_id: f[0].to_string(),
            event_id: f[1].to_string(),
            source_record_id: f[2].to_string(),
            source_id: f[3].to_string(),
            revision: parse_u32(f[4], "revision", line_number)?,
            observed_tick: parse_i64(f[5], "observed_tick", line_number)?,
            event_tick: parse_i64(f[6], "event_tick", line_number)?,
            classification: Classification::parse(f[7])?,
            state: RecordState::parse(f[8])?,
            event_kind: f[9].to_string(),
            quantity: parse_u64(f[10], "quantity", line_number)?,
            terminal_id: f[11].to_string(),
        };

        if event.revision == 0 {
            return Err(format!("line {line_number}: revision must start at one"));
        }
        events.push(event);
    }
    Ok(events)
}

fn signed_delta(ledger: LedgerKind, event_kind: &str, quantity: u64) -> Result<i128, String> {
    let q = i128::from(quantity);
    match (ledger, event_kind) {
        (LedgerKind::VesselQueue, "ELIGIBLE_ARRIVAL" | "REBOOKED_RETURN") => Ok(q),
        (
            LedgerKind::VesselQueue,
            "FIRST_BERTH_ENTRY" | "DIVERSION" | "CANCELLATION" | "OTHER_EXIT",
        ) => Ok(-q),
        (LedgerKind::ContainerStock, "DISCHARGE_IN" | "GATE_IN" | "RAIL_IN" | "ADJUSTMENT_IN") => Ok(q),
        (
            LedgerKind::ContainerStock,
            "VESSEL_LOAD" | "GATE_OUT" | "RAIL_OUT" | "ADJUSTMENT_OUT",
        ) => Ok(-q),
        (LedgerKind::ContainerStock, "REHANDLE") => Ok(0),
        (_, "NONE") => Err("actual active event cannot have event_kind NONE".to_string()),
        (_, other) => Err(format!("event kind {other:?} is unsupported for this ledger")),
    }
}

fn is_zero_state(event: &SourceEvent) -> bool {
    event.event_kind == "NONE" && event.quantity == 0
}

fn verify_case(case: &CaseDefinition, input_events: &[SourceEvent]) -> Result<u64, String> {
    let mut records_by_id: BTreeMap<String, SourceEvent> = BTreeMap::new();

    // First make exact replay idempotent and reject source-record identity reuse.
    for event in input_events.iter().filter(|event| event.case_id == case.case_id) {
        match records_by_id.get(&event.source_record_id) {
            Some(existing) if existing == event => continue,
            Some(_) => {
                return Err(format!(
                    "source_record_id {:?} was reused with different content",
                    event.source_record_id
                ));
            }
            None => {
                records_by_id.insert(event.source_record_id.clone(), event.clone());
            }
        }
    }

    let mut by_event: BTreeMap<String, Vec<SourceEvent>> = BTreeMap::new();
    for event in records_by_id.into_values() {
        if event.terminal_id != case.terminal_id {
            return Err(format!(
                "event {:?} terminal {:?} does not match case terminal {:?}",
                event.event_id, event.terminal_id, case.terminal_id
            ));
        }
        by_event.entry(event.event_id.clone()).or_default().push(event);
    }

    let mut active: Vec<SourceEvent> = Vec::new();

    for (event_id, mut revisions) in by_event {
        revisions.sort_by_key(|event| event.revision);
        let source_id = revisions[0].source_id.as_str();
        let terminal_id = revisions[0].terminal_id.as_str();

        if revisions.iter().any(|event| {\n            event.source_id != source_id\n                || event.terminal_id != terminal_id\n                || event.quantity_unit != case.stock_unit\n        }) {
            return Err(format!("event {event_id:?} changed source or terminal identity across revisions"));
        }

        let mut visible: Vec<SourceEvent> = revisions
            .into_iter()
            .filter(|event| event.observed_tick <= case.as_of_tick)
            .collect();

        if visible.is_empty() {
            // Future-observed evidence must not influence historical replay.
            continue;
        }

        visible.sort_by_key(|event| event.revision);
        for (index, event) in visible.iter().enumerate() {
            let expected_revision = u32::try_from(index + 1)
                .map_err(|_| "revision index overflow".to_string())?;
            if event.revision != expected_revision {
                return Err(format!(
                    "event {event_id:?} has a visible revision gap: expected {expected_revision}, found {}",
                    event.revision
                ));
            }
        }

        for pair in visible.windows(2) {
            if pair[0].revision == pair[1].revision {
                return Err(format!("event {event_id:?} has conflicting records for revision {}", pair[0].revision));
            }
        }

        let current = visible.last().expect("visible revisions is non-empty");
        if current.state == RecordState::Voided {
            if !is_zero_state(current) {
                return Err(format!("voided event {event_id:?} must have NONE and zero quantity"));
            }
            continue;
        }

        if current.classification != Classification::Actual {
            if !is_zero_state(current) {
                return Err(format!(
                    "non-actual {:?} event {event_id:?} cannot mutate physical stock",
                    current.classification
                ));
            }
            continue;
        }

        if current.event_tick < case.window_start_tick || current.event_tick > case.as_of_tick {
            return Err(format!("actual event {event_id:?} is outside the case time window"));
        }
        if current.observed_tick < current.event_tick {
            return Err(format!("actual event {event_id:?} was observed before it occurred"));
        }
        if current.quantity == 0 {
            return Err(format!("actual event {event_id:?} has zero quantity"));
        }

        // This validates event-kind/ledger compatibility before arithmetic.
        signed_delta(case.ledger, &current.event_kind, current.quantity)?;
        active.push(current.clone());
    }

    // The event identity, not ingestion order, defines deterministic order.
    active.sort_by(|a, b| (a.event_tick, &a.event_id).cmp(&(b.event_tick, &b.event_id)));

    let mut stock = i128::from(case.opening_stock);
    for event in &active {
        let delta = signed_delta(case.ledger, &event.event_kind, event.quantity)?;
        stock = stock
            .checked_add(delta)
            .ok_or_else(|| "stock arithmetic overflow".to_string())?;
        if stock < 0 {
            return Err(format!(
                "stock underflow after event {:?} at tick {}",
                event.event_id, event.event_tick
            ));
        }
    }

    let calculated = u64::try_from(stock)
        .map_err(|_| "calculated closing stock is outside u64".to_string())?;

    if calculated != case.reported_closing_stock {
        return Err(format!(
            "stock-flow mismatch: calculated closing stock {calculated}, reported {}",
            case.reported_closing_stock
        ));
    }

    Ok(calculated)
}

pub fn evaluate_corpus(cases_csv: &str, events_csv: &str) -> Result<CorpusReport, String> {
    let cases = parse_cases(cases_csv)?;
    let events = parse_events(events_csv)?;
    let case_ids: BTreeSet<&str> = cases.iter().map(|case| case.case_id.as_str()).collect();

    for event in &events {
        if !case_ids.contains(event.case_id.as_str()) {
            return Err(format!("event references unknown case {:?}", event.case_id));
        }
    }

    let mut results = Vec::with_capacity(cases.len());
    let mut accepted = 0;
    let mut rejected = 0;
    let mut matched = 0;

    for case in &cases {
        let result = verify_case(case, &events);
        let (actual, detail) = match result {
            Ok(closing) => ("ACCEPT".to_string(), format!("closing stock = {closing}")),
            Err(error) => ("REJECT".to_string(), error),
        };

        if actual == "ACCEPT" {
            accepted += 1;
        } else {
            rejected += 1;
        }

        let expected = if case.expected_accept { "ACCEPT" } else { "REJECT" }.to_string();
        if actual == expected {
            matched += 1;
        }

        results.push(CaseResult {
            case_id: case.case_id.clone(),
            expected,
            actual,
            detail,
        });
    }

    Ok(CorpusReport {
        accepted,
        rejected,
        matched,
        total: cases.len(),
        cases: results,
    })
}

pub fn evaluate_reference_corpus() -> Result<CorpusReport, String> {
    evaluate_corpus(CASES_CSV, EVENTS_CSV)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_thirty_frozen_dispositions_match() {
        let report = evaluate_reference_corpus().expect("frozen corpus should parse");
        assert_eq!(report.total, 30);
        assert_eq!(report.accepted, 15);
        assert_eq!(report.rejected, 15);
        assert_eq!(report.matched, 30);
        assert!(report.all_expected_dispositions_match());
    }

    #[test]
    fn exact_source_record_replay_is_idempotent() {
        let events = parse_events(EVENTS_CSV).expect("events parse");
        let cases = parse_cases(CASES_CSV).expect("cases parse");
        let case = cases.iter().find(|case| case.case_id == "V06").expect("V06 exists");
        let result = verify_case(case, &events);
        assert_eq!(result, Ok(1));
    }

    #[test]
    fn correction_replaces_earlier_revision_instead_of_adding_to_it() {
        let report = evaluate_reference_corpus().expect("corpus parses");
        let case = report.cases.iter().find(|case| case.case_id == "V05").expect("V05 exists");
        assert_eq!(case.actual, "ACCEPT");
        assert_eq!(case.detail, "closing stock = 4");
    }

    #[test]
    fn future_observed_revision_does_not_leak_into_prior_snapshot() {
        let report = evaluate_reference_corpus().expect("corpus parses");
        let case = report.cases.iter().find(|case| case.case_id == "V08").expect("V08 exists");
        assert_eq!(case.actual, "ACCEPT");
        assert_eq!(case.detail, "closing stock = 1");
    }

    #[test]
    fn voided_latest_revision_removes_prior_event_from_stock() {
        let report = evaluate_reference_corpus().expect("corpus parses");
        let case = report.cases.iter().find(|case| case.case_id == "V09").expect("V09 exists");
        assert_eq!(case.actual, "ACCEPT");
        assert_eq!(case.detail, "closing stock = 1");
    }

    #[test]
    fn rehandles_are_not_counted_as_additional_container_stock() {
        let report = evaluate_reference_corpus().expect("corpus parses");
        let case = report.cases.iter().find(|case| case.case_id == "C10").expect("C10 exists");
        assert_eq!(case.actual, "ACCEPT");
        assert_eq!(case.detail, "closing stock = 115");
    }

    #[test]
    fn event_input_permutation_does_not_change_report() {
        let mut events = parse_events(EVENTS_CSV).expect("events parse");
        events.reverse();
        let event_rows = events
            .iter()
            .map(|event| format!(
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                event.case_id,
                event.event_id,
                event.source_record_id,
                event.source_id,
                event.revision,
                event.observed_tick,
                event.event_tick,
                match event.classification {
                    Classification::Actual => "ACTUAL",
                    Classification::Estimated => "ESTIMATED",
                    Classification::Planned => "PLANNED",
                    Classification::Requested => "REQUESTED",
                },
                match event.state {
                    RecordState::Active => "ACTIVE",
                    RecordState::Voided => "VOIDED",
                },
                event.event_kind,
                event.quantity,
                event.terminal_id,
            ))
            .collect::<Vec<_>>()
            .join("\n");
        let reordered = format!("{EVENTS_HEADER}\n{event_rows}\n");
        let original = evaluate_reference_corpus().expect("original corpus");
        let permuted = evaluate_corpus(CASES_CSV, &reordered).expect("permuted corpus");
        assert_eq!(original, permuted);
    }

    #[test]
    fn source_record_id_conflict_is_rejected() {
        let events = EVENTS_CSV.replace(
            "V24,REUSED-24,REC-V24-1,OPS_A,1,10,9,ACTUAL,ACTIVE,ELIGIBLE_ARRIVAL,2,DURBAN_A",
            "V24,REUSED-24,REC-V24-1,OPS_A,1,10,9,ACTUAL,ACTIVE,ELIGIBLE_ARRIVAL,1,DURBAN_A",
        );
        let report = evaluate_corpus(CASES_CSV, &events).expect("corpus parses");
        let case = report.cases.iter().find(|case| case.case_id == "V24").expect("V24 exists");
        assert_eq!(case.actual, "REJECT");
        assert!(case.detail.contains("reused with different content"));
    }

    #[test]
    fn planned_and_estimated_events_cannot_change_actual_stock() {
        let mut events = parse_events(EVENTS_CSV).expect("events parse");
        let case = parse_cases(CASES_CSV).expect("cases parse")
            .into_iter().find(|case| case.case_id == "V07").expect("V07 exists");
        let planned = events.iter().find(|event| event.case_id == "V07" && event.classification == Classification::Planned).expect("planned event");
        assert_eq!(planned.event_kind, "NONE");
        assert_eq!(verify_case(&case, &events), Ok(1));
        events.retain(|event| !(event.case_id == "V07" && event.classification == Classification::Actual));
        assert!(verify_case(&case, &events).is_err());
    }

    #[test]
    fn container_and_vessel_units_are_not_interchangeable() {
        let report = evaluate_reference_corpus().expect("corpus parses");
        let vessel_case = report.cases.iter().find(|case| case.case_id == "V18").expect("V18 exists");
        let container_case = report.cases.iter().find(|case| case.case_id == "C19").expect("C19 exists");
        assert_eq!(vessel_case.actual, "REJECT");
        assert_eq!(container_case.actual, "REJECT");
    }

    #[test]
    fn malformed_schema_is_not_guessed() {
        let error = evaluate_corpus(
            &CASES_CSV.replace("opening_stock", "opening_quantity"),
            EVENTS_CSV,
        )
        .expect_err("schema change must reject");
        assert!(error.contains("unexpected case CSV header"));
    }
}
