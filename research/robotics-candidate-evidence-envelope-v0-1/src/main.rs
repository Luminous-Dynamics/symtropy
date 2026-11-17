use robotics_candidate_evidence_envelope::{
    CandidateBinding, CandidateEligibility, CandidateEvidenceEnvelope, EvidenceDomain,
    EvidenceReceipt, EvidenceStatus, SCHEMA_VERSION, validate_candidate_envelope,
};

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn main() {
    let binding = fixture_binding();
    let preimage_bytes = binding.canonical_binding_bytes().len();
    let receipts = fixture_receipts(&binding);
    let envelope = CandidateEvidenceEnvelope {
        schema_version: SCHEMA_VERSION.to_owned(),
        binding,
        binding_sha256: A.to_owned(),
        receipts,
    };

    match validate_candidate_envelope(&envelope, A) {
        Ok(result) => {
            println!("suite=ROBOTICS-CANDIDATE-EVIDENCE-ENVELOPE-001");
            println!("schema={SCHEMA_VERSION}");
            println!("eligibility={:?}", result.eligibility());
            println!("receipt_count={}", envelope.receipts.len());
            println!("canonical_binding_preimage_bytes={preimage_bytes}");
            println!(
                "candidate_feedable={}",
                result.eligibility() == CandidateEligibility::Qualified
            );
            println!(
                "note=fixture-only structural check; digest authentication and physical truth are external"
            );
        }
        Err(errors) => {
            eprintln!("candidate_envelope_invalid={errors:?}");
            std::process::exit(2);
        }
    }
}

fn fixture_binding() -> CandidateBinding {
    CandidateBinding {
        candidate_id: "candidate-01".to_owned(),
        pass_id: "pass-north-01".to_owned(),
        mission_id: "mission-pv-01".to_owned(),
        mission_revision: "mission-r2".to_owned(),
        scenario_id: "scenario-roof-01".to_owned(),
        scenario_revision: "scenario-r3".to_owned(),
        robot_id: "uav-01".to_owned(),
        robot_configuration_revision: "uav-config-r4".to_owned(),
        robot_configuration_sha256: A.to_owned(),
        module_manifest_sha256: B.to_owned(),
        site_id: "roof-01".to_owned(),
        site_revision: "site-r5".to_owned(),
        frame_id: "array-plane-uv".to_owned(),
        frame_revision: "frame-r3".to_owned(),
        map_revision: "map-r6".to_owned(),
        timebase_id: "flight-clock-01".to_owned(),
        timebase_revision: "clock-map-r1".to_owned(),
        acquisition_start_ns: 1_000,
        acquisition_end_ns: 2_000,
        sensor_id: "rgb-ir-payload-01".to_owned(),
        sensor_revision: "sensor-r7".to_owned(),
        mount_id: "gimbal-mount-01".to_owned(),
        mount_revision: "mount-r2".to_owned(),
        calibration_bundle_sha256: C.to_owned(),
        observation_bundle_sha256: D.to_owned(),
        target_ids: vec!["module-01".to_owned(), "module-02".to_owned()],
        position_u_m: 3.0,
        position_v_m: 4.0,
        view_angle_rad: 0.65,
        cost_model_id: "survey-cost-model".to_owned(),
        cost_model_revision: "cost-r1".to_owned(),
        cost_model_sha256: A.to_owned(),
        estimated_travel_distance_m: 20.0,
        estimated_travel_time_s: 10.0,
        estimated_acquisition_time_s: 4.0,
        estimated_energy_wh: 2.5,
        risk_penalty_s: 0.5,
    }
}

fn fixture_receipts(binding: &CandidateBinding) -> Vec<EvidenceReceipt> {
    vec![
        build_receipt(
            binding,
            EvidenceDomain::GeometryCoverage,
            "geometry-record-01",
            B,
        ),
        build_receipt(
            binding,
            EvidenceDomain::SensorQuality,
            "sensor-record-01",
            C,
        ),
        build_receipt(
            binding,
            EvidenceDomain::Calibration,
            "calibration-record-01",
            D,
        ),
    ]
}

fn build_receipt(
    binding: &CandidateBinding,
    domain: EvidenceDomain,
    record_id: &str,
    result_sha256: &str,
) -> EvidenceReceipt {
    EvidenceReceipt {
        domain,
        timebase_id: binding.timebase_id.clone(),
        timebase_revision: binding.timebase_revision.clone(),
        record_id: record_id.to_owned(),
        issuer_id: format!("evaluator-{domain:?}"),
        issuer_revision: "evaluator-r1".to_owned(),
        profile_id: format!("profile-{domain:?}"),
        profile_revision: "profile-r2".to_owned(),
        status: EvidenceStatus::Qualified,
        binding_sha256: A.to_owned(),
        result_sha256: result_sha256.to_owned(),
        source_bundle_sha256: match domain {
            EvidenceDomain::GeometryCoverage | EvidenceDomain::SensorQuality => {
                binding.observation_bundle_sha256.clone()
            }
            EvidenceDomain::Calibration => binding.calibration_bundle_sha256.clone(),
        },
        issued_at_ns: match domain {
            EvidenceDomain::GeometryCoverage | EvidenceDomain::SensorQuality => 2_500,
            EvidenceDomain::Calibration => 800,
        },
        valid_from_ns: (domain == EvidenceDomain::Calibration).then_some(500),
        valid_until_ns: (domain == EvidenceDomain::Calibration).then_some(3_000),
        covered_target_ids: if domain == EvidenceDomain::GeometryCoverage {
            binding.target_ids.clone()
        } else {
            Vec::new()
        },
    }
}
