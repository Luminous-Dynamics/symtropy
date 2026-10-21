//! Frozen, hand-calculated oracle for quantity delta/snapshot semantics and bitemporal replay.
//!
//! This fixture evaluator deliberately parses the source corpus and expected-output TSV
//! independently of the production normalizer. It does not import production aggregation helpers.

use shipyard_cost_trace::Stage;
use shipyard_cost_trace::event_semantics::{
    AsOfCutoff, NormalizedQuantity, NormalizedSemantics, QuantityIdentity, QuantityRecord,
    QuantityRecordKind, normalize_quantities,
};

const INPUTS: &str = include_str!("fixtures/event-semantics-v1.tsv");
const EXPECTED: &str = include_str!("fixtures/event-semantics-v1-expected.tsv");

fn parse_stage(value: &str) -> Stage {
    match value {
        "planned" => Stage::Planned,
        "simulated" => Stage::Simulated,
        "observed" => Stage::Observed,
        other => panic!("unexpected stage in frozen semantics corpus: {other}"),
    }
}

fn parse_case(case_id: &str) -> (AsOfCutoff, Vec<QuantityRecord>) {
    let mut cutoff = None;
    let mut records = Vec::new();

    for line in INPUTS.lines().skip(1).filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 17, "input corpus must have 17 columns");
        if fields[0] != case_id {
            continue;
        }

        let this_cutoff = AsOfCutoff {
            event_time_ms: fields[1].parse().expect("valid event-time cutoff"),
            recorded_at_ms: fields[2].parse().expect("valid recorded-time cutoff"),
        };
        if let Some(previous) = cutoff {
            assert_eq!(
                previous, this_cutoff,
                "cutoff must be stable within case {case_id}"
            );
        } else {
            cutoff = Some(this_cutoff);
        }

        let kind = match fields[14] {
            "delta" => QuantityRecordKind::Delta {
                quantity: fields[15].parse().expect("valid delta quantity"),
            },
            "snapshot" => QuantityRecordKind::CumulativeSnapshot {
                quantity: fields[15].parse().expect("valid snapshot quantity"),
            },
            "correction" => QuantityRecordKind::Correction {
                target_record_id: fields[16].to_owned(),
                replacement_quantity: fields[15].parse().expect("valid replacement quantity"),
            },
            "reversal" => QuantityRecordKind::Reversal {
                target_record_id: fields[16].to_owned(),
            },
            other => panic!("unexpected quantity record kind in frozen corpus: {other}"),
        };

        records.push(QuantityRecord {
            record_id: fields[3].to_owned(),
            stream_id: fields[4].to_owned(),
            sequence: fields[5].parse().expect("valid source sequence"),
            event_time_ms: fields[6].parse().expect("valid source event time"),
            recorded_at_ms: fields[7].parse().expect("valid system-recorded time"),
            identity: QuantityIdentity {
                build_id: fields[8].to_owned(),
                configuration_sha256: fields[9].to_owned(),
                task_id: fields[10].to_owned(),
                quantity_key: fields[11].to_owned(),
                unit: fields[12].to_owned(),
                stage: parse_stage(fields[13]),
            },
            kind,
        });
    }

    (
        cutoff.unwrap_or_else(|| panic!("no records for frozen case {case_id}")),
        records,
    )
}

fn semantics_label(value: NormalizedSemantics) -> &'static str {
    match value {
        NormalizedSemantics::AdditiveDeltas => "AdditiveDeltas",
        NormalizedSemantics::LatestCumulativeSnapshot => "LatestCumulativeSnapshot",
    }
}

fn assert_expected(actual: &[NormalizedQuantity], fields: &[&str], case_id: &str) {
    assert_eq!(actual.len(), 1, "one normalized row expected for {case_id}");
    let row = &actual[0];
    let expected_quantity: f64 = fields[1].parse().expect("valid expected quantity");
    assert_eq!(row.quantity, expected_quantity, "quantity for {case_id}");
    assert_eq!(
        semantics_label(row.semantics),
        fields[2],
        "semantics for {case_id}"
    );
    assert_eq!(
        row.evidence_record_ids.join(";"),
        fields[3],
        "evidence lineage for {case_id}"
    );
    assert_eq!(
        row.source_stream_ids.join(";"),
        fields[4],
        "source streams for {case_id}"
    );
}

#[test]
fn frozen_quantity_semantics_corpus_matches_hand_calculated_outputs() {
    let mut expected_lines = EXPECTED.lines();
    assert_eq!(
        expected_lines.next(),
        Some("case_id\tquantity\tsemantics\tevidence_record_ids\tsource_stream_ids")
    );

    let mut cases_seen = Vec::new();
    for line in expected_lines.filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 5, "expected corpus must have 5 columns");
        let (cutoff, records) = parse_case(fields[0]);
        let actual = normalize_quantities(&records, cutoff).unwrap_or_else(|errors| {
            panic!("valid frozen case {} rejected: {errors:?}", fields[0])
        });
        assert_expected(&actual, &fields, fields[0]);
        cases_seen.push(fields[0]);
    }

    assert_eq!(
        cases_seen,
        vec![
            "delta-sum",
            "corrected-deltas",
            "snapshot-asof-early",
            "snapshot-asof-late"
        ],
        "frozen case inventory must remain explicit"
    );
}
