use fusion_component_lifecycle_contract::{
    assess, ClaimCeiling, ComponentCandidate, Disposition, EvidenceClass, EvidenceRef, Gate,
    GateState,
};

const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

const GATES: [(&str, &str); 6] = [
    ("material", "material-applicability"),
    ("process", "process-capability"),
    ("tolerance", "tolerance-stack"),
    ("inspection", "inspection-access"),
    ("assembly", "assembly-access"),
    ("replacement", "replacement-access"),
];

fn parse_state(value: &str) -> GateState {
    match value {
        "P" => GateState::Pass,
        "F" => GateState::Fail,
        "U" => GateState::Unknown,
        other => panic!("unsupported gate state in frozen corpus: {other}"),
    }
}

fn make_gate(state: GateState, evidence_id: &str) -> Gate {
    match state {
        GateState::Pass => Gate {
            state,
            reason_code: None,
            evidence_ids: vec![evidence_id.to_owned()],
        },
        GateState::Fail => Gate {
            state,
            reason_code: Some("SYNTHETIC_NEGATIVE_CONTROL".to_owned()),
            evidence_ids: vec![evidence_id.to_owned()],
        },
        GateState::Unknown => Gate {
            state,
            reason_code: Some("SYNTHETIC_DOMAIN_NOT_ADMITTED".to_owned()),
            evidence_ids: vec![],
        },
    }
}

fn make_candidate(states: &[&str]) -> ComponentCandidate {
    assert_eq!(states.len(), 6);
    let evidence = GATES
        .iter()
        .enumerate()
        .map(|(index, (id, scope))| EvidenceRef {
            id: format!("ev-{id}"),
            subject_ref: "component/synthetic-article".to_owned(),
            subject_revision: "design-r1".to_owned(),
            scope_ref: (*scope).to_owned(),
            content_sha256: if index % 2 == 0 { HASH_A } else { HASH_B }.to_owned(),
            class: EvidenceClass::SyntheticFixture,
        })
        .collect::<Vec<_>>();

    ComponentCandidate {
        candidate_id: "fixture-candidate".to_owned(),
        design_revision: "design-r1".to_owned(),
        geometry_sha256: HASH_A.to_owned(),
        material_record_ref: "symthaea://materials/synthetic-mat-01".to_owned(),
        process_plan_ref: "symthaea://process-plans/synthetic-route-01".to_owned(),
        material_applicability: make_gate(parse_state(states[0]), "ev-material"),
        process_capability: make_gate(parse_state(states[1]), "ev-process"),
        tolerance_stack: make_gate(parse_state(states[2]), "ev-tolerance"),
        inspection_access: make_gate(parse_state(states[3]), "ev-inspection"),
        assembly_access: make_gate(parse_state(states[4]), "ev-assembly"),
        replacement_access: make_gate(parse_state(states[5]), "ev-replacement"),
        evidence,
    }
}

fn disposition_name(disposition: Disposition) -> &'static str {
    match disposition {
        Disposition::Blocked => "BLOCKED",
        Disposition::Indeterminate => "INDETERMINATE",
        Disposition::EligibleForFurtherEngineeringReview => "REVIEW",
    }
}

#[test]
fn frozen_fixture_corpus_matches_contract_assessments() {
    let corpus = include_str!("../fixtures/lifecycle-cases-v1.tsv");
    let mut case_count = 0usize;

    for (line_number, line) in corpus.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("case_id|") {
            continue;
        }

        let fields = line.split('|').collect::<Vec<_>>();
        assert_eq!(fields.len(), 9, "invalid fixture row at line {}", line_number + 1);
        let case_id = fields[0];
        let states = &fields[1..7];
        let mutation = fields[7];
        let expected = fields[8];
        let mut candidate = make_candidate(states);

        match mutation {
            "none" => {}
            "pass_without_evidence" => candidate.material_applicability.evidence_ids.clear(),
            "missing_reason" => {
                candidate.material_applicability.reason_code = None;
            }
            "dangling_reference" => {
                candidate.process_capability.evidence_ids = vec!["ev-missing".to_owned()];
            }
            "scope_mismatch" => {
                let reference = candidate
                    .evidence
                    .iter_mut()
                    .find(|reference| reference.id == "ev-process")
                    .expect("process evidence exists");
                reference.scope_ref = "inspection-access".to_owned();
            }
            "duplicate_id" => {
                let duplicate = candidate
                    .evidence
                    .iter()
                    .find(|reference| reference.id == "ev-process")
                    .expect("process evidence exists")
                    .clone();
                candidate.evidence.push(duplicate);
            }
            "malformed_digest" => candidate.geometry_sha256 = "not-a-sha256".to_owned(),
            other => panic!("unknown integrity mutation {other} in case {case_id}"),
        }

        let result = assess(&candidate);
        if expected == "REJECT" {
            assert!(
                result.is_err(),
                "case {case_id} expected rejection but was accepted"
            );
        } else {
            let assessment = result.unwrap_or_else(|errors| {
                panic!("case {case_id} unexpectedly rejected: {errors:?}")
            });
            assert_eq!(
                disposition_name(assessment.disposition),
                expected,
                "wrong disposition for case {case_id}"
            );
            assert_eq!(
                assessment.claim_ceiling,
                ClaimCeiling::ContractAndSyntheticFixtureOnly,
                "claim ceiling changed for case {case_id}"
            );
        }
        case_count += 1;
    }

    assert_eq!(case_count, 12, "frozen corpus case count changed");
}
