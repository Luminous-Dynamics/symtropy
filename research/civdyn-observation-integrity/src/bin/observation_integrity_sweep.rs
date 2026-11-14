// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic smoke sweep for the isolated observation-integrity crate.
//! This is a contract exerciser, not a physical sensor qualification.

use civdyn_observation_integrity::{
    AdmissionPolicy, CandidateDisposition, DependencyProfile, EvidenceClass, EvidenceKind,
    EvidenceRecord, EvidenceResult, IntegrityAuditError, triage_observation_integrity,
};

fn policy() -> AdmissionPolicy {
    AdmissionPolicy {
        scenario_id: "civdyn-observation-integrity-sweep-v0.2".to_owned(),
        source_digest: "fixture-reference-only-not-a-verified-payload-digest".to_owned(),
        expected_unit: "service_index".to_owned(),
        expected_calibration_revision: "cal-7".to_owned(),
        expected_configuration_revision: "cfg-3".to_owned(),
        maximum_age_ticks: 2,
        numeric_tolerance: 2,
    }
}

fn dependency_profile(id: &str, source: &str) -> DependencyProfile {
    DependencyProfile {
        observation_root_id: Some(format!("observation-{id}")),
        upstream_dataset_id: Some(format!("dataset-{id}")),
        processing_lineage_id: Some(format!("processing-{id}")),
        operator_domain_id: Some(format!("operator-{source}")),
        instrument_lineage_id: Some(format!("instrument-{source}")),
    }
}

fn measured(
    id: &str,
    kind: EvidenceKind,
    value: i64,
    source: &str,
    gateway: &str,
    trust_root: &str,
) -> EvidenceRecord {
    EvidenceRecord {
        evidence_id: id.to_owned(),
        kind,
        class: EvidenceClass::Measured,
        result: EvidenceResult::Numeric(value),
        unit: "service_index".to_owned(),
        event_tick: 10,
        ingestion_tick: 10,
        source_id: source.to_owned(),
        gateway_id: gateway.to_owned(),
        trust_root_id: trust_root.to_owned(),
        dependencies: dependency_profile(id, source),
        calibration_revision: "cal-7".to_owned(),
        configuration_revision: "cfg-3".to_owned(),
        authenticated: true,
    }
}

fn primary() -> EvidenceRecord {
    measured(
        "primary",
        EvidenceKind::PrimaryTelemetry,
        50,
        "sensor-a",
        "gateway-a",
        "trust-root-a",
    )
}

fn disposition_name(value: CandidateDisposition) -> &'static str {
    value.id()
}

fn check_disposition(
    label: &str,
    evidence: &[EvidenceRecord],
    expected: CandidateDisposition,
) -> bool {
    match triage_observation_integrity(&policy(), evidence) {
        Ok(decision) => {
            let passed = decision.disposition == expected;
            println!(
                "{}\t{}\t{}",
                if passed { "PASS" } else { "FAIL" },
                label,
                disposition_name(decision.disposition)
            );
            passed
        }
        Err(error) => {
            println!("FAIL\t{}\terror:{:?}", label, error);
            false
        }
    }
}

fn check_error(label: &str, evidence: &[EvidenceRecord], expected: IntegrityAuditError) -> bool {
    match triage_observation_integrity(&policy(), evidence) {
        Err(actual) => {
            let passed = actual == expected;
            println!(
                "{}\t{}\terror:{:?}",
                if passed { "PASS" } else { "FAIL" },
                label,
                actual
            );
            passed
        }
        Ok(decision) => {
            println!(
                "FAIL\t{}\tunexpected:{}",
                label,
                disposition_name(decision.disposition)
            );
            false
        }
    }
}

fn run_sweep() -> usize {
    let mut failures = 0usize;
    let mut cases = 0usize;

    macro_rules! disposition_case {
        ($label:expr, $evidence:expr, $expected:expr) => {{
            cases += 1;
            if !check_disposition($label, &$evidence, $expected) {
                failures += 1;
            }
        }};
    }
    macro_rules! error_case {
        ($label:expr, $evidence:expr, $expected:expr) => {{
            cases += 1;
            if !check_error($label, &$evidence, $expected) {
                failures += 1;
            }
        }};
    }

    // A single unusual telemetry value does not identify its physical cause.
    disposition_case!(
        "telemetry-only-abstains",
        vec![primary()],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    // A genuinely separate physical cross-check may corroborate the value,
    // but still cannot attribute why the primary telemetry was anomalous.
    disposition_case!(
        "independent-agreement-corroborates",
        vec![
            primary(),
            measured(
                "independent",
                EvidenceKind::IndependentPhysicalMeasurement,
                51,
                "sensor-b",
                "gateway-b",
                "trust-root-b",
            ),
        ],
        CandidateDisposition::UseCorroboratedMeasurement
    );

    disposition_case!(
        "independent-disagreement-preserves-conflict",
        vec![
            primary(),
            measured(
                "independent-conflict",
                EvidenceKind::IndependentPhysicalMeasurement,
                90,
                "sensor-b",
                "gateway-b",
                "trust-root-b",
            ),
        ],
        CandidateDisposition::EvidenceConflict
    );

    // Independence must not be inferred from different sensor IDs alone.
    disposition_case!(
        "shared-source-is-not-independent",
        vec![
            primary(),
            measured(
                "same-source",
                EvidenceKind::IndependentPhysicalMeasurement,
                50,
                "sensor-a",
                "gateway-b",
                "trust-root-b",
            ),
        ],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    disposition_case!(
        "shared-gateway-is-not-independent",
        vec![
            primary(),
            measured(
                "same-gateway",
                EvidenceKind::IndependentPhysicalMeasurement,
                50,
                "sensor-b",
                "gateway-a",
                "trust-root-b",
            ),
        ],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    disposition_case!(
        "shared-trust-root-is-not-independent",
        vec![
            primary(),
            measured(
                "same-root",
                EvidenceKind::IndependentPhysicalMeasurement,
                50,
                "sensor-b",
                "gateway-b",
                "trust-root-a",
            ),
        ],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    let mut broker_copy = measured(
        "broker-copy",
        EvidenceKind::IndependentPhysicalMeasurement,
        50,
        "broker-b",
        "gateway-b",
        "trust-root-b",
    );
    broker_copy.dependencies.observation_root_id =
        primary().dependencies.observation_root_id;
    disposition_case!(
        "different-brokers-same-observation-is-correlated",
        vec![primary(), broker_copy],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    let mut mirror_copy = measured(
        "mirror-copy",
        EvidenceKind::IndependentPhysicalMeasurement,
        50,
        "national-mirror",
        "gateway-mirror",
        "trust-root-mirror",
    );
    mirror_copy.dependencies.upstream_dataset_id =
        primary().dependencies.upstream_dataset_id;
    disposition_case!(
        "different-gateways-same-upstream-dataset-is-correlated",
        vec![primary(), mirror_copy],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    let mut same_pipeline = measured(
        "same-pipeline",
        EvidenceKind::IndependentPhysicalMeasurement,
        50,
        "sensor-c",
        "gateway-c",
        "trust-root-c",
    );
    same_pipeline.dependencies.processing_lineage_id =
        primary().dependencies.processing_lineage_id;
    disposition_case!(
        "same-processing-lineage-is-correlated",
        vec![primary(), same_pipeline],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    let mut unknown_lineage = measured(
        "unknown-lineage",
        EvidenceKind::IndependentPhysicalMeasurement,
        50,
        "sensor-d",
        "gateway-d",
        "trust-root-d",
    );
    unknown_lineage.dependencies.upstream_dataset_id = None;
    disposition_case!(
        "unknown-dependency-does-not-upgrade-to-independent",
        vec![primary(), unknown_lineage],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    let mut stale_primary = primary();
    stale_primary.event_tick = 1;
    disposition_case!(
        "excessive-event-to-ingestion-lag-is-rejected",
        vec![stale_primary],
        CandidateDisposition::RejectEvidence
    );

    let mut wrong_calibration = primary();
    wrong_calibration.calibration_revision = "cal-old".to_owned();
    disposition_case!(
        "wrong-calibration-is-rejected",
        vec![wrong_calibration],
        CandidateDisposition::RejectEvidence
    );

    let mut unauthenticated = primary();
    unauthenticated.authenticated = false;
    disposition_case!(
        "unauthenticated-primary-is-rejected",
        vec![unauthenticated],
        CandidateDisposition::RejectEvidence
    );

    disposition_case!(
        "derived-reconstruction-does-not-corroborate-itself",
        vec![
            primary(),
            EvidenceRecord {
                evidence_id: "reconstruction".to_owned(),
                kind: EvidenceKind::DerivedReconstruction,
                class: EvidenceClass::Reconstructed,
                result: EvidenceResult::Numeric(50),
                unit: "service_index".to_owned(),
                event_tick: 10,
                ingestion_tick: 10,
                source_id: "model".to_owned(),
                gateway_id: "gateway-model".to_owned(),
                trust_root_id: "trust-root-model".to_owned(),
                dependencies: dependency_profile("reconstruction", "model"),
                calibration_revision: "cal-7".to_owned(),
                configuration_revision: "cfg-3".to_owned(),
                authenticated: true,
            },
        ],
        CandidateDisposition::AbstainObservationalEquivalence
    );

    disposition_case!(
        "independent-self-test-failure-requests-measurement",
        vec![
            primary(),
            EvidenceRecord {
                evidence_id: "self-test".to_owned(),
                kind: EvidenceKind::InstrumentSelfTest,
                class: EvidenceClass::Attested,
                result: EvidenceResult::SelfTestFail,
                unit: "service_index".to_owned(),
                event_tick: 10,
                ingestion_tick: 10,
                source_id: "self-test-system".to_owned(),
                gateway_id: "gateway-test".to_owned(),
                trust_root_id: "trust-root-test".to_owned(),
                dependencies: dependency_profile("self-test", "self-test-system"),
                calibration_revision: "cal-7".to_owned(),
                configuration_revision: "cfg-3".to_owned(),
                authenticated: true,
            },
        ],
        CandidateDisposition::RequestIndependentPhysicalMeasurement
    );

    disposition_case!(
        "independent-forensic-indicator-requests-integrity-investigation",
        vec![
            primary(),
            EvidenceRecord {
                evidence_id: "forensic".to_owned(),
                kind: EvidenceKind::ForensicIntegrityReview,
                class: EvidenceClass::Attested,
                result: EvidenceResult::IntegrityIndicator,
                unit: "service_index".to_owned(),
                event_tick: 10,
                ingestion_tick: 10,
                source_id: "forensic-reviewer".to_owned(),
                gateway_id: "gateway-forensic".to_owned(),
                trust_root_id: "trust-root-forensic".to_owned(),
                dependencies: dependency_profile("forensic", "forensic-reviewer"),
                calibration_revision: "cal-7".to_owned(),
                configuration_revision: "cfg-3".to_owned(),
                authenticated: true,
            },
        ],
        CandidateDisposition::IntegrityInvestigation
    );

    error_case!(
        "duplicate-evidence-id-is-rejected",
        vec![primary(), primary()],
        IntegrityAuditError::DuplicateEvidenceId
    );

    error_case!(
        "missing-primary-is-rejected",
        vec![measured(
            "side-only",
            EvidenceKind::IndependentPhysicalMeasurement,
            50,
            "sensor-b",
            "gateway-b",
            "trust-root-b",
        )],
        IntegrityAuditError::MissingPrimaryTelemetry
    );

    let mut clock_inversion = primary();
    clock_inversion.ingestion_tick = 9;
    error_case!(
        "ingestion-before-event-is-rejected",
        vec![clock_inversion],
        IntegrityAuditError::InvalidClockOrder
    );

    // The evaluator result is deterministic under evidence permutation.
    cases += 1;
    let first = vec![
        primary(),
        measured(
            "independent",
            EvidenceKind::IndependentPhysicalMeasurement,
            51,
            "sensor-b",
            "gateway-b",
            "trust-root-b",
        ),
    ];
    let mut reversed = first.clone();
    reversed.reverse();
    let order_invariant = triage_observation_integrity(&policy(), &first)
        .ok()
        .zip(triage_observation_integrity(&policy(), &reversed).ok())
        .is_some_and(|(left, right)| left == right);
    println!(
        "{}\tevidence-order-invariance\t{}",
        if order_invariant { "PASS" } else { "FAIL" },
        if order_invariant {
            "identical-decision"
        } else {
            "decision-differs"
        }
    );
    if !order_invariant {
        failures += 1;
    }

    println!(
        "SUMMARY\tcases={}\tpassed={}\tfailed={}",
        cases,
        cases.saturating_sub(failures),
        failures
    );
    failures
}

fn main() {
    if run_sweep() != 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_sweep_passes() {
        assert_eq!(run_sweep(), 0);
    }

    #[test]
    fn agreeing_measurement_does_not_claim_cause_attribution() {
        let evidence = vec![
            primary(),
            measured(
                "independent",
                EvidenceKind::IndependentPhysicalMeasurement,
                50,
                "sensor-b",
                "gateway-b",
                "trust-root-b",
            ),
        ];
        let decision = triage_observation_integrity(&policy(), &evidence).unwrap();
        assert_eq!(
            decision.disposition,
            CandidateDisposition::UseCorroboratedMeasurement
        );
        assert!(
            decision
                .unresolved_hypotheses
                .iter()
                .any(|value| value.contains("unattributed"))
        );
    }

    #[test]
    fn common_cause_dimensions_are_independently_tested() {
        let cases = [
            ("same-source", "sensor-a", "gateway-b", "trust-root-b"),
            ("same-gateway", "sensor-b", "gateway-a", "trust-root-b"),
            ("same-root", "sensor-b", "gateway-b", "trust-root-a"),
        ];
        for (id, source, gateway, root) in cases {
            let evidence = vec![
                primary(),
                measured(
                    id,
                    EvidenceKind::IndependentPhysicalMeasurement,
                    50,
                    source,
                    gateway,
                    root,
                ),
            ];
            let decision = triage_observation_integrity(&policy(), &evidence).unwrap();
            assert_eq!(
                decision.disposition,
                CandidateDisposition::AbstainObservationalEquivalence,
                "common-cause case {id} must not corroborate"
            );
        }
    }
}
