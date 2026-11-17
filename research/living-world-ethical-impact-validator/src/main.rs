// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

fn main() {
    match living_world_ethical_impact_validator::validate_builtin_corpus() {
        Ok(summary) => {
            println!(
                "PASS: {} fixtures, {} profiles, {} premises, {} anti-derivation controls, {} evidence references, {} action references",
                summary.fixture_count,
                summary.profile_count,
                summary.premise_count,
                summary.anti_derivation_control_count,
                summary.evidence_reference_count,
                summary.action_reference_count
            );
        }
        Err(errors) => {
            eprintln!("FAIL: Living World Ethical Impact corpus conformance");
            for error in errors {
                eprintln!("- {error}");
            }
            std::process::exit(1);
        }
    }
}
