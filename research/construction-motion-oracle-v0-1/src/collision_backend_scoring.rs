//! Independent scorer for backend-only collision observations.
//!
//! The adapter reports execution and observation. This module compares that
//! record with evaluator-owned analytic truth and emits the only TP/FP/FN/TN
//! classification. It does not parse JSON or verify that supplied digests
//! match source bytes; the adapter runner must perform those checks before
//! constructing these typed records.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryMode {
    Discrete,
    LvsDiscrete,
    LvsContinuous,
    Continuous,
    Analytic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryScope {
    RobotWorld,
    SelfCollision,
    Analytic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObstacleMotion {
    Static,
    Moving,
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryRepresentation {
    NativePrimitive,
    Mesh,
    ConvexHull,
    SphereApproximation,
    CapsuleApproximation,
    Mixed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseIdentity {
    pub case_id: String,
    pub case_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneFingerprint {
    pub robot_model_sha256: String,
    pub tool_sha256: String,
    pub payload_sha256: String,
    pub obstacle_set_sha256: String,
    pub geometry_profile_sha256: String,
    pub frame_id: String,
    pub transform_sha256: String,
    pub trajectory_sha256: String,
    pub interpolation_profile_sha256: String,
    pub collision_matrix_sha256: String,
    pub margin_profile_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QueryFingerprint {
    pub mode: QueryMode,
    pub scope: QueryScope,
    pub obstacle_motion: ObstacleMotion,
    pub geometry_representation: GeometryRepresentation,
    /// Required for LVS modes; expressed in millimetres.
    pub longest_valid_segment_mm: Option<f64>,
    pub contact_tolerance_mm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BackendIdentity {
    pub backend_name: String,
    pub backend_version: String,
    pub backend_commit_sha1: String,
    pub build_sha256: String,
    pub runtime: String,
    pub collision_plugin: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CapabilityProbeStatus {
    Executed,
    Unsupported,
    Error,
    NotProbed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityProbe {
    pub probe_id: String,
    pub status: CapabilityProbeStatus,
    pub runtime_api: Option<String>,
    pub probe_log_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterDisposition {
    Observed,
    Unresolved,
    NotRun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterReasonCode {
    BackendUnsupported,
    CapabilityUnproven,
    SemanticMismatch,
    Timeout,
    ProcessError,
    MalformedOutput,
    MissingOutput,
    InvalidInput,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionObservation {
    pub collision_found: bool,
    pub minimum_distance_mm: Option<f64>,
    pub contact_count: Option<u64>,
    pub backend_contact_time_parameter: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterMetrics {
    pub latency_ns: Option<u64>,
    pub peak_rss_bytes: Option<u64>,
    pub allocated_bytes: Option<u64>,
    pub broad_phase_candidates: Option<u64>,
    pub narrow_phase_queries: Option<u64>,
    pub manager_clone_count: Option<u64>,
    pub evaluator_count: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvironmentIdentity {
    pub operating_system: String,
    pub architecture: String,
    pub cpu_model: String,
    pub thread_count: u32,
    pub allocator: Option<String>,
    pub build_profile: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdapterObservation {
    pub run_id: String,
    pub case: CaseIdentity,
    pub backend: BackendIdentity,
    pub scene: SceneFingerprint,
    pub query: QueryFingerprint,
    pub capability_probe: CapabilityProbe,
    pub disposition: AdapterDisposition,
    pub observation: Option<CollisionObservation>,
    pub reason_code: Option<AdapterReasonCode>,
    pub reason: Option<String>,
    pub metrics: Option<AdapterMetrics>,
    pub environment: EnvironmentIdentity,
    pub raw_receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OracleTruthDisposition {
    Contact,
    NoContact,
    Unsupported,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OracleTruth {
    pub case: CaseIdentity,
    pub scene: SceneFingerprint,
    pub query: QueryFingerprint,
    pub disposition: OracleTruthDisposition,
    pub truth_receipt_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScorerIdentity {
    pub scorer_name: String,
    pub scorer_version: String,
    pub scorer_binary_sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfusionCell {
    TruePositive,
    FalsePositive,
    FalseNegative,
    TrueNegative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreDisposition {
    Matched,
    Unresolved,
    NotRun,
    NotComparable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScoreReasonCode {
    AdapterNotRun,
    AdapterUnresolved,
    OracleUnknown,
    OracleUnsupported,
    CaseDigestMismatch,
    SceneFingerprintMismatch,
    QuerySemanticsMismatch,
    GeometryProfileMismatch,
    ReceiptInvalid,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScoreVerdict {
    pub case_id: String,
    pub case_sha256: String,
    pub adapter_raw_receipt_sha256: String,
    pub oracle_truth_receipt_sha256: String,
    pub scorer_name: String,
    pub scorer_version: String,
    pub scorer_binary_sha256: String,
    pub disposition: ScoreDisposition,
    pub confusion_cell: Option<ConfusionCell>,
    pub reason_code: Option<ScoreReasonCode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CollisionScoreInputError {
    EmptyIdentity,
    InvalidDigest(&'static str),
    InvalidBackendIdentity,
    InvalidEnvironmentIdentity,
    InvalidProbeIdentity,
    InvalidObservation,
    InvalidQueryFingerprint,
    CapabilityNotExecuted,
    MissingObservation,
    UnexpectedObservation,
    MissingReason,
    UnexpectedReason,
    ContradictoryNotRunProbe,
}

pub fn score_backend_observation(
    adapter: &AdapterObservation,
    truth: &OracleTruth,
    scorer: &ScorerIdentity,
) -> Result<ScoreVerdict, CollisionScoreInputError> {
    validate_adapter(adapter)?;
    validate_truth(truth)?;
    validate_scorer(scorer)?;

    let mut verdict = ScoreVerdict {
        case_id: adapter.case.case_id.clone(),
        case_sha256: adapter.case.case_sha256.clone(),
        adapter_raw_receipt_sha256: adapter.raw_receipt_sha256.clone(),
        oracle_truth_receipt_sha256: truth.truth_receipt_sha256.clone(),
        scorer_name: scorer.scorer_name.clone(),
        scorer_version: scorer.scorer_version.clone(),
        scorer_binary_sha256: scorer.scorer_binary_sha256.clone(),
        disposition: ScoreDisposition::NotComparable,
        confusion_cell: None,
        reason_code: None,
    };

    match adapter.disposition {
        AdapterDisposition::NotRun => {
            verdict.disposition = ScoreDisposition::NotRun;
            verdict.reason_code = Some(ScoreReasonCode::AdapterNotRun);
            return Ok(verdict);
        }
        AdapterDisposition::Unresolved => {
            verdict.disposition = ScoreDisposition::Unresolved;
            verdict.reason_code = Some(ScoreReasonCode::AdapterUnresolved);
            return Ok(verdict);
        }
        AdapterDisposition::Observed => {}
    }

    if adapter.case.case_id != truth.case.case_id {
        verdict.reason_code = Some(ScoreReasonCode::CaseDigestMismatch);
        return Ok(verdict);
    }
    if adapter.case.case_sha256 != truth.case.case_sha256 {
        verdict.reason_code = Some(ScoreReasonCode::CaseDigestMismatch);
        return Ok(verdict);
    }
    if !scene_fingerprints_match(&adapter.scene, &truth.scene) {
        verdict.reason_code = Some(if adapter.scene.geometry_profile_sha256
            != truth.scene.geometry_profile_sha256
            || adapter.scene.robot_model_sha256 != truth.scene.robot_model_sha256
            || adapter.scene.tool_sha256 != truth.scene.tool_sha256
            || adapter.scene.payload_sha256 != truth.scene.payload_sha256
            || adapter.scene.obstacle_set_sha256 != truth.scene.obstacle_set_sha256
        {
            ScoreReasonCode::GeometryProfileMismatch
        } else {
            ScoreReasonCode::SceneFingerprintMismatch
        });
        return Ok(verdict);
    }
    if !query_fingerprints_match(adapter.query, truth.query) {
        verdict.reason_code = Some(ScoreReasonCode::QuerySemanticsMismatch);
        return Ok(verdict);
    }

    match truth.disposition {
        OracleTruthDisposition::Unsupported => {
            verdict.disposition = ScoreDisposition::NotComparable;
            verdict.reason_code = Some(ScoreReasonCode::OracleUnsupported);
        }
        OracleTruthDisposition::Unknown => {
            verdict.disposition = ScoreDisposition::Unresolved;
            verdict.reason_code = Some(ScoreReasonCode::OracleUnknown);
        }
        OracleTruthDisposition::Contact | OracleTruthDisposition::NoContact => {
            let observation = adapter
                .observation
                .ok_or(CollisionScoreInputError::MissingObservation)?;
            let cell = match (truth.disposition, observation.collision_found) {
                (OracleTruthDisposition::Contact, true) => ConfusionCell::TruePositive,
                (OracleTruthDisposition::Contact, false) => ConfusionCell::FalseNegative,
                (OracleTruthDisposition::NoContact, true) => ConfusionCell::FalsePositive,
                (OracleTruthDisposition::NoContact, false) => ConfusionCell::TrueNegative,
                (OracleTruthDisposition::Unsupported | OracleTruthDisposition::Unknown, _) => {
                    return Err(CollisionScoreInputError::InvalidObservation);
                }
            };
            verdict.disposition = ScoreDisposition::Matched;
            verdict.confusion_cell = Some(cell);
        }
    }

    Ok(verdict)
}

fn validate_adapter(adapter: &AdapterObservation) -> Result<(), CollisionScoreInputError> {
    if adapter.run_id.trim().is_empty()
        || adapter.case.case_id.trim().is_empty()
        || adapter.scene.frame_id.trim().is_empty()
        || adapter.environment.operating_system.trim().is_empty()
        || adapter.environment.architecture.trim().is_empty()
        || adapter.environment.cpu_model.trim().is_empty()
        || adapter.environment.build_profile.trim().is_empty()
        || adapter.environment.thread_count == 0
    {
        return Err(CollisionScoreInputError::EmptyIdentity);
    }

    validate_digest(adapter.case.case_sha256.as_str(), "case")?;
    validate_digest_with_length(adapter.backend.backend_commit_sha1.as_str(), "backend commit", 40)?;
    validate_digest(adapter.backend.build_sha256.as_str(), "backend build")?;
    validate_digest(adapter.raw_receipt_sha256.as_str(), "raw receipt")?;

    if [
        adapter.backend.backend_name.as_str(),
        adapter.backend.backend_version.as_str(),
        adapter.backend.runtime.as_str(),
        adapter.backend.collision_plugin.as_str(),
    ]
    .into_iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(CollisionScoreInputError::InvalidBackendIdentity);
    }

    if adapter.capability_probe.probe_id.trim().is_empty() {
        return Err(CollisionScoreInputError::InvalidProbeIdentity);
    }
    if let Some(api) = adapter.capability_probe.runtime_api.as_deref() {
        if api.trim().is_empty() {
            return Err(CollisionScoreInputError::InvalidProbeIdentity);
        }
    }
    if let Some(digest) = adapter.capability_probe.probe_log_sha256.as_deref() {
        validate_digest(digest, "capability probe log")?;
    }
    if adapter.capability_probe.status != CapabilityProbeStatus::NotProbed
        && (adapter.capability_probe.runtime_api.as_deref().is_none_or(str::is_empty)
            || adapter.capability_probe.probe_log_sha256.is_none())
    {
        return Err(CollisionScoreInputError::InvalidProbeIdentity);
    }

    if adapter.environment.allocator.as_deref().is_some_and(str::is_empty) {
        return Err(CollisionScoreInputError::InvalidEnvironmentIdentity);
    }

    validate_scene_fingerprint(&adapter.scene)?;
    validate_query_fingerprint(adapter.query)?;

    match adapter.disposition {
        AdapterDisposition::Observed => {
            if adapter.capability_probe.status != CapabilityProbeStatus::Executed {
                return Err(CollisionScoreInputError::CapabilityNotExecuted);
            }
            if adapter.observation.is_none() {
                return Err(CollisionScoreInputError::MissingObservation);
            }
            if adapter.reason.is_some() || adapter.reason_code.is_some() {
                return Err(CollisionScoreInputError::UnexpectedReason);
            }
        }
        AdapterDisposition::Unresolved => {
            if adapter.observation.is_some() {
                return Err(CollisionScoreInputError::UnexpectedObservation);
            }
            if adapter.reason.as_deref().is_none_or(str::is_empty)
                || adapter.reason_code.is_none()
            {
                return Err(CollisionScoreInputError::MissingReason);
            }
        }
        AdapterDisposition::NotRun => {
            if adapter.observation.is_some() {
                return Err(CollisionScoreInputError::UnexpectedObservation);
            }
            if adapter.reason.as_deref().is_none_or(str::is_empty)
                || adapter.reason_code.is_none()
            {
                return Err(CollisionScoreInputError::MissingReason);
            }
            if !matches!(
                adapter.capability_probe.status,
                CapabilityProbeStatus::Unsupported | CapabilityProbeStatus::NotProbed
            ) {
                return Err(CollisionScoreInputError::ContradictoryNotRunProbe);
            }
        }
    }

    if let Some(observation) = adapter.observation {
        validate_collision_observation(observation)?;
    }


    Ok(())
}

fn validate_truth(truth: &OracleTruth) -> Result<(), CollisionScoreInputError> {
    if truth.case.case_id.trim().is_empty() || truth.scene.frame_id.trim().is_empty() {
        return Err(CollisionScoreInputError::EmptyIdentity);
    }
    validate_digest(truth.case.case_sha256.as_str(), "oracle case")?;
    validate_digest(truth.truth_receipt_sha256.as_str(), "oracle truth receipt")?;
    validate_scene_fingerprint(&truth.scene)?;
    validate_query_fingerprint(truth.query)?;
    Ok(())
}

fn validate_scorer(scorer: &ScorerIdentity) -> Result<(), CollisionScoreInputError> {
    if scorer.scorer_name.trim().is_empty() || scorer.scorer_version.trim().is_empty() {
        return Err(CollisionScoreInputError::EmptyIdentity);
    }
    validate_digest(scorer.scorer_binary_sha256.as_str(), "scorer binary")
}

fn validate_scene_fingerprint(
    scene: &SceneFingerprint,
) -> Result<(), CollisionScoreInputError> {
    for (value, name) in [
        (scene.robot_model_sha256.as_str(), "robot model"),
        (scene.tool_sha256.as_str(), "tool"),
        (scene.payload_sha256.as_str(), "payload"),
        (scene.obstacle_set_sha256.as_str(), "obstacle set"),
        (scene.geometry_profile_sha256.as_str(), "geometry profile"),
        (scene.transform_sha256.as_str(), "transform"),
        (scene.trajectory_sha256.as_str(), "trajectory"),
        (scene.interpolation_profile_sha256.as_str(), "interpolation profile"),
        (scene.collision_matrix_sha256.as_str(), "collision matrix"),
        (scene.margin_profile_sha256.as_str(), "margin profile"),
    ] {
        validate_digest(value, name)?;
    }
    if scene.frame_id.trim().is_empty() {
        return Err(CollisionScoreInputError::EmptyIdentity);
    }
    Ok(())
}

fn validate_query_fingerprint(query: QueryFingerprint) -> Result<(), CollisionScoreInputError> {
    if !query.contact_tolerance_mm.is_finite() || query.contact_tolerance_mm < 0.0 {
        return Err(CollisionScoreInputError::InvalidQueryFingerprint);
    }
    match (query.mode, query.longest_valid_segment_mm) {
        (QueryMode::LvsDiscrete | QueryMode::LvsContinuous, Some(value))
            if value.is_finite() && value > 0.0 => {}
        (QueryMode::LvsDiscrete | QueryMode::LvsContinuous, _) => {
            return Err(CollisionScoreInputError::InvalidQueryFingerprint);
        }
        (_, Some(_)) => return Err(CollisionScoreInputError::InvalidQueryFingerprint),
        (_, None) => {}
    }
    if (query.mode == QueryMode::Analytic) != (query.scope == QueryScope::Analytic) {
        return Err(CollisionScoreInputError::InvalidQueryFingerprint);
    }
    Ok(())
}

fn validate_collision_observation(
    observation: CollisionObservation,
) -> Result<(), CollisionScoreInputError> {
    if observation
        .minimum_distance_mm
        .is_some_and(|value| !value.is_finite())
        || observation
            .backend_contact_time_parameter
            .is_some_and(|value| !value.is_finite() || !(0.0..=1.0).contains(&value))
        || observation
            .contact_count
            .is_some_and(|count| count == 0 && observation.collision_found)
        || observation
            .contact_count
            .is_some_and(|count| count > 0 && !observation.collision_found)
    {
        return Err(CollisionScoreInputError::InvalidObservation);
    }
    Ok(())
}

fn scene_fingerprints_match(left: &SceneFingerprint, right: &SceneFingerprint) -> bool {
    left == right
}

fn query_fingerprints_match(left: QueryFingerprint, right: QueryFingerprint) -> bool {
    left.mode == right.mode
        && left.scope == right.scope
        && left.obstacle_motion == right.obstacle_motion
        && left.geometry_representation == right.geometry_representation
        && option_f64_matches(
            left.longest_valid_segment_mm,
            right.longest_valid_segment_mm,
        )
        && left.contact_tolerance_mm.to_bits() == right.contact_tolerance_mm.to_bits()
}

fn option_f64_matches(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        _ => false,
    }
}

fn validate_digest(value: &str, name: &'static str) -> Result<(), CollisionScoreInputError> {
    validate_digest_with_length(value, name, 64)
}

fn validate_digest_with_length(
    value: &str,
    _name: &'static str,
    expected_length: usize,
) -> Result<(), CollisionScoreInputError> {
    if value.len() != expected_length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CollisionScoreInputError::InvalidDigest(_name));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn scene() -> SceneFingerprint {
        SceneFingerprint {
            robot_model_sha256: digest('a'),
            tool_sha256: digest('b'),
            payload_sha256: digest('c'),
            obstacle_set_sha256: digest('d'),
            geometry_profile_sha256: digest('e'),
            frame_id: "site-world".into(),
            transform_sha256: digest('f'),
            trajectory_sha256: digest('1'),
            interpolation_profile_sha256: digest('2'),
            collision_matrix_sha256: digest('3'),
            margin_profile_sha256: digest('4'),
        }
    }

    fn query() -> QueryFingerprint {
        QueryFingerprint {
            mode: QueryMode::LvsContinuous,
            scope: QueryScope::RobotWorld,
            obstacle_motion: ObstacleMotion::Static,
            geometry_representation: GeometryRepresentation::ConvexHull,
            longest_valid_segment_mm: Some(25.0),
            contact_tolerance_mm: 0.1,
        }
    }

    fn adapter(collision_found: bool) -> AdapterObservation {
        AdapterObservation {
            run_id: "run-1".into(),
            case: CaseIdentity {
                case_id: "thin-wall".into(),
                case_sha256: digest('5'),
            },
            backend: BackendIdentity {
                backend_name: "Tesseract".into(),
                backend_version: "0.35.0".into(),
                backend_commit_sha1: "0123456789abcdef0123456789abcdef01234567".into(),
                build_sha256: digest('6'),
                runtime: "standalone".into(),
                collision_plugin: "BulletCastBVHManager".into(),
            },
            scene: scene(),
            query: query(),
            capability_probe: CapabilityProbe {
                probe_id: "continuous-contact-test".into(),
                status: CapabilityProbeStatus::Executed,
                runtime_api: Some("contactTest".into()),
                probe_log_sha256: Some(digest('7')),
            },
            disposition: AdapterDisposition::Observed,
            observation: Some(CollisionObservation {
                collision_found,
                minimum_distance_mm: Some(if collision_found { -0.2 } else { 2.5 }),
                contact_count: Some(if collision_found { 1 } else { 0 }),
                backend_contact_time_parameter: Some(0.5).filter(|_| collision_found),
            }),
            reason_code: None,
            reason: None,
            metrics: None,
            environment: EnvironmentIdentity {
                operating_system: "linux".into(),
                architecture: "x86_64".into(),
                cpu_model: "test-cpu".into(),
                thread_count: 1,
                allocator: Some("system".into()),
                build_profile: "release".into(),
            },
            raw_receipt_sha256: digest('8'),
        }
    }

    fn truth(disposition: OracleTruthDisposition) -> OracleTruth {
        OracleTruth {
            case: adapter(false).case,
            scene: scene(),
            query: query(),
            disposition,
            truth_receipt_sha256: digest('9'),
        }
    }

    fn scorer() -> ScorerIdentity {
        ScorerIdentity {
            scorer_name: "construction-motion-scorer".into(),
            scorer_version: "0.2".into(),
            scorer_binary_sha256: digest('a'),
        }
    }

    #[test]
    fn emits_all_four_confusion_cells_from_oracle_truth_and_observation() {
        let cases = [
            (OracleTruthDisposition::Contact, true, ConfusionCell::TruePositive),
            (OracleTruthDisposition::Contact, false, ConfusionCell::FalseNegative),
            (OracleTruthDisposition::NoContact, true, ConfusionCell::FalsePositive),
            (OracleTruthDisposition::NoContact, false, ConfusionCell::TrueNegative),
        ];
        for (oracle, observed, expected) in cases {
            let verdict = score_backend_observation(&adapter(observed), &truth(oracle), &scorer())
                .unwrap();
            assert_eq!(verdict.disposition, ScoreDisposition::Matched);
            assert_eq!(verdict.confusion_cell, Some(expected));
        }
    }

    #[test]
    fn not_run_and_unresolved_never_become_true_negatives() {
        let mut not_run = adapter(false);
        not_run.disposition = AdapterDisposition::NotRun;
        not_run.observation = None;
        not_run.reason_code = Some(AdapterReasonCode::BackendUnsupported);
        not_run.reason = Some("continuous overload unsupported".into());
        not_run.capability_probe.status = CapabilityProbeStatus::Unsupported;
        let verdict =
            score_backend_observation(&not_run, &truth(OracleTruthDisposition::NoContact), &scorer())
                .unwrap();
        assert_eq!(verdict.disposition, ScoreDisposition::NotRun);
        assert_eq!(verdict.confusion_cell, None);

        let mut unresolved = not_run.clone();
        unresolved.disposition = AdapterDisposition::Unresolved;
        unresolved.capability_probe.status = CapabilityProbeStatus::Error;
        unresolved.reason_code = Some(AdapterReasonCode::ProcessError);
        let verdict = score_backend_observation(
            &unresolved,
            &truth(OracleTruthDisposition::NoContact),
            &scorer(),
        )
        .unwrap();
        assert_eq!(verdict.disposition, ScoreDisposition::Unresolved);
        assert_eq!(verdict.confusion_cell, None);
    }

    #[test]
    fn scene_and_query_mismatches_are_not_comparable() {
        let mut adapter = adapter(false);
        adapter.scene.transform_sha256 = digest('0');
        let verdict = score_backend_observation(
            &adapter,
            &truth(OracleTruthDisposition::NoContact),
            &scorer(),
        )
        .unwrap();
        assert_eq!(verdict.disposition, ScoreDisposition::NotComparable);
        assert_eq!(
            verdict.reason_code,
            Some(ScoreReasonCode::SceneFingerprintMismatch)
        );

        let mut adapter = adapter(false);
        adapter.query.longest_valid_segment_mm = Some(50.0);
        let verdict = score_backend_observation(
            &adapter,
            &truth(OracleTruthDisposition::NoContact),
            &scorer(),
        )
        .unwrap();
        assert_eq!(
            verdict.reason_code,
            Some(ScoreReasonCode::QuerySemanticsMismatch)
        );
    }

    #[test]
    fn false_negative_cannot_be_claimed_by_the_adapter() {
        let mut adapter = adapter(false);
        adapter.disposition = AdapterDisposition::NotRun;
        adapter.observation = None;
        adapter.reason_code = Some(AdapterReasonCode::BackendUnsupported);
        adapter.reason = Some("not executed".into());
        adapter.capability_probe.status = CapabilityProbeStatus::NotProbed;
        let verdict = score_backend_observation(
            &adapter,
            &truth(OracleTruthDisposition::Contact),
            &scorer(),
        )
        .unwrap();
        assert_ne!(verdict.confusion_cell, Some(ConfusionCell::FalseNegative));
    }

    #[test]
    fn invalid_lvs_or_capability_claims_fail_closed() {
        let mut invalid_query = adapter(true);
        invalid_query.query.longest_valid_segment_mm = None;
        assert_eq!(
            score_backend_observation(
                &invalid_query,
                &truth(OracleTruthDisposition::Contact),
                &scorer()
            ),
            Err(CollisionScoreInputError::InvalidQueryFingerprint)
        );

        let mut unproven = adapter(true);
        unproven.capability_probe.status = CapabilityProbeStatus::NotProbed;
        assert_eq!(
            score_backend_observation(&unproven, &truth(OracleTruthDisposition::Contact), &scorer()),
            Err(CollisionScoreInputError::CapabilityNotExecuted)
        );
    }
    #[test]
    fn oracle_abstentions_and_unsupported_truth_never_form_confusion_cells() {
        let observed = adapter(false);
        let unknown = score_backend_observation(
            &observed,
            &truth(OracleTruthDisposition::Unknown),
            &scorer(),
        )
        .unwrap();
        assert_eq!(unknown.disposition, ScoreDisposition::Unresolved);
        assert_eq!(unknown.confusion_cell, None);
        assert_eq!(unknown.reason_code, Some(ScoreReasonCode::OracleUnknown));

        let unsupported = score_backend_observation(
            &observed,
            &truth(OracleTruthDisposition::Unsupported),
            &scorer(),
        )
        .unwrap();
        assert_eq!(unsupported.disposition, ScoreDisposition::NotComparable);
        assert_eq!(unsupported.confusion_cell, None);
        assert_eq!(unsupported.reason_code, Some(ScoreReasonCode::OracleUnsupported));
    }

    #[test]
    fn mismatched_case_identity_and_collision_scope_are_not_comparable() {
        let mut mismatched_case = truth(OracleTruthDisposition::Contact);
        mismatched_case.case.case_sha256 = digest('0');
        let verdict =
            score_backend_observation(&adapter(true), &mismatched_case, &scorer()).unwrap();
        assert_eq!(verdict.disposition, ScoreDisposition::NotComparable);
        assert_eq!(
            verdict.reason_code,
            Some(ScoreReasonCode::CaseDigestMismatch)
        );

        let mut self_collision_truth = truth(OracleTruthDisposition::Contact);
        self_collision_truth.query.scope = QueryScope::SelfCollision;
        let verdict = score_backend_observation(
            &adapter(true),
            &self_collision_truth,
            &scorer(),
        )
        .unwrap();
        assert_eq!(verdict.disposition, ScoreDisposition::NotComparable);
        assert_eq!(
            verdict.reason_code,
            Some(ScoreReasonCode::QuerySemanticsMismatch)
        );
    }

    #[test]
    fn malformed_receipts_and_inconsistent_not_run_status_are_rejected() {
        let mut malformed = adapter(true);
        malformed.raw_receipt_sha256 = "not-a-digest".into();
        assert_eq!(
            score_backend_observation(
                &malformed,
                &truth(OracleTruthDisposition::Contact),
                &scorer()
            ),
            Err(CollisionScoreInputError::InvalidDigest("raw receipt"))
        );

        let mut inconsistent_not_run = adapter(false);
        inconsistent_not_run.disposition = AdapterDisposition::NotRun;
        inconsistent_not_run.observation = None;
        inconsistent_not_run.reason_code = Some(AdapterReasonCode::BackendUnsupported);
        inconsistent_not_run.reason = Some("not supported".into());
        // An executed capability probe contradicts NOT_RUN.
        assert_eq!(
            score_backend_observation(
                &inconsistent_not_run,
                &truth(OracleTruthDisposition::NoContact),
                &scorer()
            ),
            Err(CollisionScoreInputError::ContradictoryNotRunProbe)
        );
    }

    #[test]
    fn observed_receipt_requires_probe_log_and_runtime_api() {
        let mut missing_probe_log = adapter(true);
        missing_probe_log.capability_probe.probe_log_sha256 = None;
        assert_eq!(
            score_backend_observation(
                &missing_probe_log,
                &truth(OracleTruthDisposition::Contact),
                &scorer()
            ),
            Err(CollisionScoreInputError::InvalidProbeIdentity)
        );

        let mut missing_api = adapter(true);
        missing_api.capability_probe.runtime_api = None;
        assert_eq!(
            score_backend_observation(
                &missing_api,
                &truth(OracleTruthDisposition::Contact),
                &scorer()
            ),
            Err(CollisionScoreInputError::InvalidProbeIdentity)
        );
    }

}
