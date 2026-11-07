//! Independent direct interval oracle for the channel headway verifier.
//!
//! This test deliberately computes the clearance gap from the earlier end to
//! the later start; it does not call the production interval-separation helper.

use maritime_channel_conflict_contract::{
    ChannelMovement, ChannelSegmentRule, ChannelWorkload, DirectionPermission, DirectionWindow,
    EvidenceReceipt, EvidenceState, evaluate_channel_workload, TrafficDisposition,
};
use maritime_service_chain_contract::TickWindow;
use maritime_service_chain_fleet::OperationPlacement;

fn evidence(id: &str, subject: &str) -> EvidenceReceipt {
    EvidenceReceipt {
        evidence_id: id.to_owned(),
        source_id: format!("source-{id}"),
        source_revision: "source-r1".to_owned(),
        subject_id: subject.to_owned(),
        subject_revision: "route-r1".to_owned(),
        state: EvidenceState::Current,
        valid_from_tick: 0,
        valid_through_tick: 100,
        claim_ceiling: "independent synthetic headway oracle".to_owned(),
    }
}

fn movement(id: &str, start: u64, end: u64) -> ChannelMovement {
    ChannelMovement {
        movement_id: id.to_owned(),
        segment_id: "segment-1".to_owned(),
        route_revision: "route-r1".to_owned(),
        from_node_id: "node-a".to_owned(),
        to_node_id: "node-b".to_owned(),
        operation: OperationPlacement {
            demand_id: format!("demand-{id}"),
            participant_id: format!("participant-{id}"),
            alternative_id: "alternative-1".to_owned(),
            segment_id: "segment-1".to_owned(),
            start_tick: start,
            end_tick: end,
            resource_uses: Vec::new(),
        },
    }
}

fn workload(start_a: u64, end_a: u64, start_b: u64, end_b: u64, minimum_gap: u64) -> ChannelWorkload {
    ChannelWorkload {
        corridor_id: "corridor-1".to_owned(),
        time_basis_id: "ticks-1".to_owned(),
        route_revision: "route-r1".to_owned(),
        decision_tick: 0,
        segments: vec![ChannelSegmentRule {
            segment_id: "segment-1".to_owned(),
            from_node_id: "node-a".to_owned(),
            to_node_id: "node-b".to_owned(),
            route_revision: "route-r1".to_owned(),
            same_segment_min_separation_ticks: minimum_gap,
            headway_evidence: evidence("headway", "segment-1"),
            direction_windows: vec![DirectionWindow {
                window: TickWindow {
                    start_tick: 0,
                    end_tick: 100,
                },
                permission: DirectionPermission::ForwardOnly,
                evidence: evidence("direction", "segment-1"),
            }],
        }],
        interactions: Vec::new(),
        movements: vec![
            movement("m-a", start_a, end_a),
            movement("m-b", start_b, end_b),
        ],
    }
}

fn direct_reference(start_a: u64, end_a: u64, start_b: u64, end_b: u64, minimum_gap: u64) -> bool {
    if end_a <= start_b {
        start_b - end_a >= minimum_gap
    } else if end_b <= start_a {
        start_a - end_b >= minimum_gap
    } else {
        false
    }
}

#[test]
fn production_headway_result_matches_independent_interval_reference_over_512_cases() {
    let mut checked = 0;
    for start_a in 1..=8 {
        let end_a = start_a + 2;
        for gap in 0..=7 {
            let start_b = end_a + gap;
            let end_b = start_b + 2;
            for minimum_gap in 0..=7 {
                let expected = direct_reference(start_a, end_a, start_b, end_b, minimum_gap);
                let actual = evaluate_channel_workload(&workload(
                    start_a,
                    end_a,
                    start_b,
                    end_b,
                    minimum_gap,
                ))
                .expect("valid generated channel workload");
                assert_eq!(
                    actual.disposition == TrafficDisposition::ReadyForComparison,
                    expected,
                    "mismatch: first=({start_a},{end_a}), second=({start_b},{end_b}), gap={gap}, minimum={minimum_gap}"
                );
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 512);
}
