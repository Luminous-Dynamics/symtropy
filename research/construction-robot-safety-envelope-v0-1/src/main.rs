// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use construction_robot_safety_envelope::{Disposition, evaluate, frozen_cases};

fn main() {
    let cases = frozen_cases();
    let mut mismatches = 0usize;
    let mut false_safe = 0usize;

    println!("case_id|expected|actual|reason_count");
    for case in &cases {
        let decision = evaluate(&case.policy, &case.observation);
        if decision.disposition != case.expected {
            mismatches += 1;
        }
        if case.expected != Disposition::WithinSyntheticEnvelope
            && decision.disposition == Disposition::WithinSyntheticEnvelope
        {
            false_safe += 1;
        }
        println!(
            "{}|{:?}|{:?}|{}",
            case.id,
            case.expected,
            decision.disposition,
            decision.reasons.len()
        );
    }

    println!(
        "summary|cases={}|mismatches={}|false_safe={}",
        cases.len(),
        mismatches,
        false_safe
    );

    if mismatches > 0 || false_safe > 0 {
        std::process::exit(1);
    }
}
