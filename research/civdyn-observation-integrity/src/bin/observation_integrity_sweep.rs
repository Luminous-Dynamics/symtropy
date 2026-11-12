// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_observation_integrity::{
    AdmissionPolicy, CandidateDecision, EvidenceClass, EvidenceKind, EvidenceRecord,
    EvidenceResult, ForensicFinding, ReceiptStatus, SelfTestFinding, triage_observation_integrity,
};

#[derive(Clone)]
struct EvaluatorCase {
    scenario_id: &'static str,
    hidden_cause: &'static str,
    evidence: Vec<EvidenceRecord>,
}

fn policy(scenario_id: &str) -> AdmissionPolicy {
    AdmissionPolicy {
        scenario_id: scenario_id.to_owned(),
        source_digest: format!("sha256-{scenario_id}-scenario"),
        assessment_tick: 1_100,
        expected_unit: "service_index".to_owned(),
        expected_calibration_revision: "cal-7".to_owned(),
        expected_configuration_revision: "asset-cfg-3".to_owned(),
        maximum_age_ticks: 200,
        numeric_tolerance: 2,
    }
}

fn physical(id: &str, value: i64) -> EvidenceRecord {
    EvidenceRecord {
        evidence_id: id.to_owned(),
        kind: EvidenceKind::PrimaryTelemetry,
        class: EvidenceClass::Measured,
        result: EvidenceResult::Numeric(value),
        unit: "service_index".to_owned(),
        event_tick: 900,
        measurement_tick: 950,
        ingestion_tick: 1_000,
        maximum_age_ticks: 200,
        source_id: format!("source-{id}"),
        source_digest: format!("sha256-source-{id}"),
        acquisition_path_id: format!("path-{id}"),
        gateway_id: format!("gateway-{id}"),
        trust_root_id: format!("root-{id}"),
        time_root_id: format!("time-{id}"),
        calibration_authority_id: format!("cal-authority-{id}"),
        calibration_revision: "cal-7".to_owned(),
        configuration_revision: "asset-cfg-3".to_owned(),
        receipt_status: ReceiptStatus::Verified,
    }
}

fn independent_measurement(id: &str, value: i64) -> EvidenceRecord {
    let mut evidence = physical(id, value);
    evidence.kind = EvidenceKind::IndependentPhysicalMeasurement;
    evidence
}

fn self_test() -> EvidenceRecord {
    let mut evidence = physical("instrument-self-test", 0);
    evidence.kind = EvidenceKind::InstrumentSelfTest;
    evidence.class = EvidenceClass::Attested;
    evidence.result = EvidenceResult::SelfTestFail;
    evidence
}

fn forensic_review() -> EvidenceRecord {
    let mut evidence = physical("forensic-review", 0);
    evidence.kind = EvidenceKind::ForensicIntegrityReview;
    evidence.class = EvidenceClass::Attested;
    evidence.result = EvidenceResult::IntegrityIndicator;
    evidence
}

fn derived_measurement() -> EvidenceRecord {
    let mut evidence = physical("model-reconstruction", 50);
    evidence.kind = EvidenceKind::DerivedReconstruction;
    evidence.class = EvidenceClass::Reconstructed;
    evidence.result = EvidenceResult::Numeric(50);
    evidence
}

fn cases() -> Vec<EvaluatorCase> {
    let same_value = vec![physical("primary", 50)];

    let mut stale = physical("primary", 50);
    stale.event_tick = 100;
    stale.measurement_tick = 120;
    stale.ingestion_tick = 150;
    stale.maximum_age_ticks = 200;

    let mut shared_gateway = independent_measurement("field-gauge", 50);
    shared_gateway.gateway_id = "gateway-primary".to_owned();
    shared_gateway.time_root_id = "time-primary".to_owned();

    let mut wrong_calibration = physical("primary", 50);
    wrong_calibration.calibration_revision = "cal-old".to_owned();

    let mut unverified = physical("primary", 50);
    unverified.receipt_status = ReceiptStatus::Unverified;

    vec![
        EvaluatorCase {
            scenario_id: "same_value_physical_cause",
            hidden_cause: "physical_degradation",
            evidence: same_value.clone(),
        },
        EvaluatorCase {
            scenario_id: "same_value_instrument_bias",
            hidden_cause: "instrument_bias",
            evidence: same_value.clone(),
        },
        EvaluatorCase {
            scenario_id: "same_value_in_path_alteration",
            hidden_cause: "in_path_alteration",
            evidence: same_value,
        },
        EvaluatorCase {
            scenario_id: "stale_but_authenticated",
            hidden_cause: "stale_authentic_measurement",
            evidence: vec![stale],
        },
        EvaluatorCase {
            scenario_id: "shared_gateway_false_independence",
            hidden_cause: "shared_gateway_fault_domain",
            evidence: vec![physical("primary", 50), shared_gateway],
        },
        EvaluatorCase {
            scenario_id: "derived_value_not_direct_measurement",
            hidden_cause: "model_reconstruction_only",
            evidence: vec![physical("primary", 50), derived_measurement()],
        },
        EvaluatorCase {
            scenario_id: "independent_physical_corroboration",
            hidden_cause: "physical_degradation_confirmed",
            evidence: vec![physical("primary", 50), independent_measurement("field-gauge", 51)],
        },
        EvaluatorCase {
            scenario_id: "independent_measurement_conflict",
            hidden_cause: "unresolved_sensor_or_physical_conflict",
            evidence: vec![physical("primary", 50), independent_measurement("field-gauge", 100)],
        },
        EvaluatorCase {
            scenario_id: "instrument_self_test_failure",
            hidden_cause: "instrument_fault_candidate",
            evidence: vec![physical("primary", 50), self_test()],
        },
        EvaluatorCase {
            scenario_id: "independent_forensic_integrity_review",
            hidden_cause: "integrity_anomaly_under_review",
            evidence: vec![physical("primary", 50), forensic_review()],
        },
        EvaluatorCase {
            scenario_id: "wrong_calibration_revision",
            hidden_cause: "calibration_revision_mismatch",
            evidence: vec![wrong_calibration],
        },
        EvaluatorCase {
            scenario_id: "unverified_receipt",
            hidden_cause: "receipt_not_verified",
            evidence: vec![unverified],
        },
    ]
}

fn csv(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn candidate_row(scenario_id: &str, decision: &CandidateDecision) -> String {
    format!(
        "{},{},{},{},{}",
        csv(scenario_id),
        decision.disposition.id(),
        decision.action.id(),
        csv(&decision.unresolved_hypotheses.join(";")),
        csv(&decision.reason_codes.join(";"))
    )
}

fn main() -> Result<(), String> {
    let cases = cases();
    let candidate_only = std::env::args().nth(1).as_deref() == Some("--candidate-only");

    if candidate_only {
        println!("scenario_id,disposition,action,unresolved_hypotheses,reason_codes");
        for case in &cases {
            let decision = triage_observation_integrity(&policy(case.scenario_id), &case.evidence)
                .map_err(|error| format!("{error:?}"))?;
            println!("{}", candidate_row(case.scenario_id, &decision));
        }
        return Ok(());
    }

    println!(
        "scenario_id,hidden_evaluator_cause,disposition,action,primary_value,admitted_evidence_ids,rejected_evidence_ids,unresolved_hypotheses,reason_codes"
    );
    for case in &cases {
        let decision = triage_observation_integrity(&policy(case.scenario_id), &case.evidence)
            .map_err(|error| format!("{error:?}"))?;
        let value = decision.primary_value.map_or_else(String::new, |v| v.to_string());
        println!(
            "{},{},{},{},{},{},{},{},{}",
            csv(case.scenario_id),
            csv(case.hidden_cause),
            decision.disposition.id(),
            decision.action.id(),
            value,
            csv(&decision.admitted_evidence_ids.join(";")),
            csv(&decision.rejected_evidence_ids.join(";")),
            csv(&decision.unresolved_hypotheses.join(";")),
            csv(&decision.reason_codes.join(";"))
        );
    }
    Ok(())
}
