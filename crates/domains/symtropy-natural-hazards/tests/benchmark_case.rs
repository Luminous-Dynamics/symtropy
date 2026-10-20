// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Integration checks for the versioned synthetic benchmark case fixture.
//! Passing these tests validates the contract serialization, not hydrodynamic physics.

use symtropy_natural_hazards::benchmark_case::{BenchmarkCase, CASE_SCHEMA_V1, ClaimTier};

const SYNTHETIC_CASE: &str =
    include_str!("../../../../docs/research/natural-hazards/cases/flood-lake-at-rest-v1.json");

#[test]
fn synthetic_case_manifest_deserializes_and_respects_claim_ceiling() {
    let case: BenchmarkCase = serde_json::from_str(SYNTHETIC_CASE)
        .expect("the synthetic flood benchmark case must be valid JSON and match the Rust schema");

    assert_eq!(case.schema, CASE_SCHEMA_V1);
    assert_eq!(case.claim_ceiling, ClaimTier::AnalyticalOrManufactured);
    assert_eq!(case.validate(), Ok(()));
    assert!(
        case.limitations
            .iter()
            .any(|limitation| limitation.contains("synthetic")),
        "the fixture must preserve its synthetic-only claim limit"
    );
    assert!(
        !case.evidence.iter().any(|record| {
            record.class == symtropy_natural_hazards::EvidenceClass::HeldOutObservation
        }),
        "synthetic fixture must not assert held-out observational evidence"
    );
}

#[test]
fn case_json_roundtrip_preserves_declared_constraints() {
    let case: BenchmarkCase =
        serde_json::from_str(SYNTHETIC_CASE).expect("valid synthetic benchmark fixture");
    let json = serde_json::to_string(&case).expect("case serializes");
    let reparsed: BenchmarkCase = serde_json::from_str(&json).expect("case round-trips");

    assert_eq!(reparsed, case);
    assert_eq!(reparsed.input_digest_sha256, case.input_digest_sha256);
    assert_eq!(reparsed.config_digest_sha256, case.config_digest_sha256);
    assert_eq!(reparsed.claim_ceiling, ClaimTier::AnalyticalOrManufactured);
}
