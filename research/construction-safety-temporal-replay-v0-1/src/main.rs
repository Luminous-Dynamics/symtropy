// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use construction_safety_temporal_replay::{evaluate, frozen_trace_cases, Disposition};

fn main() {
    let cases = frozen_trace_cases();
    let mut mismatches = 0usize;
    let mut false_clear = 0usize;

    println!("trace_id|expected|actual|reason");
    for case in &cases {
        let decision = evaluate(&case.policy, &case.events);
        if decision.disposition != case.expected {
            mismatches += 1;
        }
        if case.expected != Disposition::WithinSyntheticEnvelope
            && decision.disposition == Disposition::WithinSyntheticEnvelope
        {
            false_clear += 1;
        }
        println!(
            "{}|{:?}|{:?}|{:?}",
            case.id, case.expected, decision.disposition, decision.reason
        );
    }

    println!(
        "summary|traces={}|mismatches={}|false_clear={}",
        cases.len(),
        mismatches,
        false_clear
    );

    if mismatches > 0 || false_clear > 0 {
        std::process::exit(1);
    }
}
