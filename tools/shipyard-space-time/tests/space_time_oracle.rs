//! Fixture-driven check of the frozen, hand-enumerated space-time benchmark corpus.
//!
//! Expected violation codes live in a separate TSV and are compared to the verifier output.
//! This validates the verifier against fixed outcomes; the unit tests also use a separate
//! integer-cell occupancy oracle for rectangle intersection and a tick-scan oracle for time.

use std::collections::BTreeMap;

use shipyard_space_time::{
    ExclusionZone, Footprint, Placement, Rect, Scenario, TimeRange, Transfer, ViolationCode, verify,
};

const FIXTURE: &str = include_str!("fixtures/space-time-v0.tsv");

#[derive(Default)]
struct FixtureCase {
    scenario: Scenario,
    expected_counts: Option<BTreeMap<String, usize>>,
}

fn parse_footprint(value: &str) -> Footprint {
    if let Some(encoded) = value.strip_prefix("rect:") {
        let parts: Vec<&str> = encoded.split(',').collect();
        assert_eq!(parts.len(), 4, "rectangle has four coordinates: {value}");
        Footprint::AxisAlignedRect(Rect {
            min_x: parts[0].parse().expect("valid min_x"),
            min_y: parts[1].parse().expect("valid min_y"),
            max_x: parts[2].parse().expect("valid max_x"),
            max_y: parts[3].parse().expect("valid max_y"),
        })
    } else if let Some(profile) = value.strip_prefix("unsupported:") {
        Footprint::Unsupported {
            profile: profile.to_owned(),
        }
    } else {
        panic!("unknown footprint encoding: {value}");
    }
}

fn parse_interval(fields: &[&str]) -> TimeRange {
    TimeRange {
        start_tick: fields[5].parse().expect("valid start tick"),
        end_tick: fields[6].parse().expect("valid end tick"),
    }
}

fn parse_expected(value: &str) -> BTreeMap<String, usize> {
    if value == "-" {
        return BTreeMap::new();
    }
    value
        .split(';')
        .map(|item| {
            let (code, count) = item
                .split_once('=')
                .unwrap_or_else(|| panic!("expected CODE=count, got {item}"));
            (
                code.to_owned(),
                count.parse().expect("valid expected violation count"),
            )
        })
        .collect()
}

fn expected_counts(report: &shipyard_space_time::VerificationReport) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for violation in &report.violations {
        *counts
            .entry(violation.code.as_str().to_owned())
            .or_insert(0) += 1;
    }
    counts
}

#[test]
fn frozen_space_time_corpus_matches_expected_violation_counts() {
    let mut lines = FIXTURE.lines();
    assert_eq!(
        lines.next(),
        Some(
            "case_id\tkind\tid\towner_id\tfootprint\tstart_tick\tend_tick\tvehicle_id\tpredecessor_ids\texpected_violation_counts"
        ),
        "frozen fixture header"
    );

    let mut cases: BTreeMap<String, FixtureCase> = BTreeMap::new();
    for (line_offset, line) in lines.filter(|line| !line.is_empty()).enumerate() {
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(
            fields.len(),
            10,
            "fixture row {} must have ten columns",
            line_offset + 2
        );
        let case = cases.entry(fields[0].to_owned()).or_default();

        match fields[1] {
            "yard" => {
                let bounds = parse_footprint(fields[4]);
                match bounds {
                    Footprint::AxisAlignedRect(rect) => case.scenario.yard_bounds = Some(rect),
                    Footprint::Unsupported { .. } => {
                        panic!("frozen yard boundary must use the supported rectangle profile")
                    }
                }
            }
            "placement" => case.scenario.placements.push(Placement {
                reservation_id: fields[2].to_owned(),
                block_id: fields[3].to_owned(),
                footprint: parse_footprint(fields[4]),
                occupied: parse_interval(&fields),
            }),
            "transfer" => {
                let predecessor_move_ids = if fields[8] == "-" {
                    Vec::new()
                } else {
                    fields[8].split(';').map(str::to_owned).collect()
                };
                case.scenario.transfers.push(Transfer {
                    id: fields[2].to_owned(),
                    block_id: fields[3].to_owned(),
                    corridor: parse_footprint(fields[4]),
                    interval: parse_interval(&fields),
                    vehicle_id: fields[7].to_owned(),
                    predecessor_move_ids,
                });
            }
            "zone" => case.scenario.exclusion_zones.push(ExclusionZone {
                id: fields[2].to_owned(),
                footprint: parse_footprint(fields[4]),
                active: parse_interval(&fields),
            }),
            "expected" => {
                assert!(
                    case.expected_counts.is_none(),
                    "case {} has duplicate expected rows",
                    fields[0]
                );
                case.expected_counts = Some(parse_expected(fields[9]));
            }
            other => panic!(
                "unknown fixture record type on row {}: {other}",
                line_offset + 2
            ),
        }
    }

    assert_eq!(cases.len(), 17, "frozen scenario inventory changed");
    for (case_id, case) in cases {
        let expected = case
            .expected_counts
            .unwrap_or_else(|| panic!("case {case_id} has no expected row"));
        let report = verify(&case.scenario);
        assert_eq!(
            expected_counts(&report),
            expected,
            "unexpected violations for frozen case {case_id}: {:?}",
            report.violations
        );
    }
}

#[test]
fn frozen_corpus_covers_both_rejection_and_feasible_cases() {
    let mut expected_codes = Vec::new();
    let mut feasible_cases = 0;
    for line in FIXTURE.lines().skip(1).filter(|line| !line.is_empty()) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields[1] == "expected" {
            if fields[9] == "-" {
                feasible_cases += 1;
            } else {
                expected_codes.extend(
                    fields[9]
                        .split(';')
                        .filter_map(|item| item.split_once('=').map(|(code, _)| code)),
                );
            }
        }
    }

    for required in [
        ViolationCode::BlockFootprintOverlap.as_str(),
        ViolationCode::BlockDoubleBooked.as_str(),
        ViolationCode::MoveBlockedByBlock.as_str(),
        ViolationCode::VehicleOverlap.as_str(),
        ViolationCode::PrecedenceViolation.as_str(),
        ViolationCode::MissingPredecessor.as_str(),
        ViolationCode::BlockInsideExclusion.as_str(),
        ViolationCode::MoveIntersectsExclusion.as_str(),
        ViolationCode::UnsupportedFootprint.as_str(),
        ViolationCode::InvalidTimeRange.as_str(),
        ViolationCode::InvalidFootprint.as_str(),
        ViolationCode::OutOfBoundsPlacement.as_str(),
        ViolationCode::OutOfBoundsTransferCorridor.as_str(),
        ViolationCode::MissingYardBounds.as_str(),
    ] {
        assert!(
            expected_codes.contains(&required),
            "frozen corpus lacks required negative control {required}"
        );
    }
    assert!(
        feasible_cases >= 3,
        "include feasible and half-open boundary controls"
    );
}
