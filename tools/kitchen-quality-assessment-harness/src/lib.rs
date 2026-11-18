// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Focused harness for the production food-quality assessment module.
//! The production implementation is linked by local path dependency.

pub use symtropy_kitchen_planner::food_quality_assessment;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_protocol_manifest_is_explicitly_unexecuted() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../../crates/domains/symtropy-kitchen-planner/schemas/fixtures/food-quality-v1.json"
        ))
        .unwrap();
        assert_eq!(manifest["status"], "synthetic_only_unexecuted");
        assert_eq!(manifest["safety_gate_is_independent"], true);
        assert_eq!(manifest["score_policy"]["serving_permit"], "never");
        let cases = manifest["scenarios"].as_array().unwrap();
        assert!(cases.len() >= 12);
        let mut ids = std::collections::BTreeSet::new();
        for scenario in cases {
            let id = scenario["id"].as_str().unwrap();
            assert!(ids.insert(id), "duplicate scenario {id}");
        }
    }

    #[test]
    fn production_module_is_linked_and_claim_ceiling_is_fixed() {
        assert_eq!(
            food_quality_assessment::FOOD_QUALITY_CLAIM_CEILING,
            "Descriptive evidence assessment for the exact declared samples and protocol only; not food-safety or allergen qualification, nutrition/medical suitability, universal taste, regulatory conformity, a serving permit, or product superiority."
        );
    }
}
