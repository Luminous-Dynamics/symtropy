//! Task-scoped commissioning for federated manufacturing digital twins.
//!
//! This crate evaluates a declared task profile against source-component
//! evidence. It deliberately does not create a universal "twin health" flag.
//! A component may be ready for one task and unready for another.
//!
//! V0 is a typed, synthetic contract evaluator. It is not an ISO conformance
//! implementation, a sensor calibration tool, a robot controller or a
//! physical validation result.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadinessStatus {
    Ready,
    Degraded,
    NotReady,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    InvalidTaskProfile,
    UnsupportedTaskProfile,
    EmptyRequirements,
    DuplicateRequirementId,
    DuplicateComponentIdentity,
    UnboundComponent,
    MissingRequiredComponent,
    MissingOptionalComponent,
    InvalidComponentIdentity,
    MissingSourceDigest,
    MissingTransformReceipt,
    OwnerMismatch,
    ComponentRevisionMismatch,
    AuthorityMismatch,
    OutputScopeMismatch,
    ClockDomainMissing,
    ObservationTimeUnknown,
    ObservationFromFuture,
    ObservationTooOld,
    SampleIntervalUnknown,
    SampleIntervalZero,
    SampleTooSlow,
    UpdateIntervalUnknown,
    UpdateIntervalZero,
    UpdateTooSlow,
    LatencyUnknown,
    LatencyExceeded,
    ClockMappingUnknown,
    ClockUncertaintyExceeded,
    CoordinateFrameUnknown,
    CoordinateFrameMismatch,
    RegistrationErrorUnknown,
    RegistrationErrorExceeded,
    CoverageUnknown,
    CoverageOutOfRange,
    CoverageBelowMinimum,
    OcclusionUnknown,
    OcclusionOutOfRange,
    OcclusionExceeded,
    FidelityUnknown,
    FidelityExceeded,
    CommonCauseDependency,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentEvidence {
    pub component_id: String,
    pub revision: String,
    pub owner: String,
    pub authority: String,
    pub output_scope: String,
    pub source_sha256: [u8; 32],
    pub transform_receipt_sha256: [u8; 32],
    pub clock_domain: String,
    pub observed_at_ns: Option<u64>,
    pub sample_interval_ns: Option<u64>,
    pub update_interval_ns: Option<u64>,
    pub p99_latency_ns: Option<u64>,
    pub clock_mapping_uncertainty_ns: Option<u64>,
    pub coordinate_frame: Option<String>,
    pub registration_error_um: Option<u64>,
    pub coverage_permille: Option<u16>,
    pub occlusion_permille: Option<u16>,
    pub fidelity_error_units: Option<u64>,
    pub common_cause_domains: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentRequirement {
    pub component_id: String,
    pub owner: String,
    pub expected_revision: String,
    pub authority: String,
    pub output_scope: String,
    pub required: bool,
    pub maximum_sample_interval_ns: Option<u64>,
    pub maximum_update_interval_ns: Option<u64>,
    pub maximum_data_age_ns: Option<u64>,
    pub maximum_p99_latency_ns: Option<u64>,
    pub maximum_clock_uncertainty_ns: Option<u64>,
    pub expected_coordinate_frame: Option<String>,
    pub maximum_registration_error_um: Option<u64>,
    pub minimum_coverage_permille: Option<u16>,
    pub maximum_occlusion_permille: Option<u16>,
    pub maximum_fidelity_error_units: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskProfile {
    pub task_id: String,
    pub revision: String,
    pub clock_domain: String,
    pub supported: bool,
    pub independent_sources_required: bool,
    pub requirements: Vec<ComponentRequirement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessFailure {
    pub component_id: String,
    pub reason: FailureReason,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessReceipt {
    pub task_id: String,
    pub profile_revision: String,
    pub evaluated_at_ns: u64,
    pub status: ReadinessStatus,
    pub evaluated_components: Vec<String>,
    pub failures: Vec<ReadinessFailure>,
    /// Stable FNV-1a 64-bit reproducibility fingerprint, not a security hash.
    pub fingerprint: u64,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn missing_or_zero_digest(value: &[u8; 32]) -> bool {
    value.iter().all(|byte| *byte == 0)
}

fn add_failure(
    failures: &mut Vec<ReadinessFailure>,
    component_id: &str,
    reason: FailureReason,
    required: bool,
) {
    failures.push(ReadinessFailure {
        component_id: component_id.to_owned(),
        reason,
        required,
    });
}

fn check_component(
    requirement: &ComponentRequirement,
    component: &ComponentEvidence,
    profile: &TaskProfile,
    evaluated_at_ns: u64,
    failures: &mut Vec<ReadinessFailure>,
) {
    let id = requirement.component_id.as_str();
    let required = requirement.required;

    if !nonempty(&component.component_id)
        || !nonempty(&component.revision)
        || !nonempty(&component.owner)
        || !nonempty(&component.authority)
        || !nonempty(&component.output_scope)
    {
        add_failure(
            failures,
            id,
            FailureReason::InvalidComponentIdentity,
            required,
        );
    }
    if missing_or_zero_digest(&component.source_sha256) {
        add_failure(failures, id, FailureReason::MissingSourceDigest, required);
    }
    if missing_or_zero_digest(&component.transform_receipt_sha256) {
        add_failure(
            failures,
            id,
            FailureReason::MissingTransformReceipt,
            required,
        );
    }
    if component.owner != requirement.owner {
        add_failure(failures, id, FailureReason::OwnerMismatch, required);
    }
    if component.revision != requirement.expected_revision {
        add_failure(
            failures,
            id,
            FailureReason::ComponentRevisionMismatch,
            required,
        );
    }
    if component.authority != requirement.authority {
        add_failure(failures, id, FailureReason::AuthorityMismatch, required);
    }
    if component.output_scope != requirement.output_scope {
        add_failure(failures, id, FailureReason::OutputScopeMismatch, required);
    }
    if !nonempty(&component.clock_domain) {
        add_failure(failures, id, FailureReason::ClockDomainMissing, required);
    }

    if let Some(observed_at_ns) = component.observed_at_ns {
        if observed_at_ns > evaluated_at_ns {
            add_failure(failures, id, FailureReason::ObservationFromFuture, required);
        } else if let Some(maximum_age_ns) = requirement.maximum_data_age_ns {
            if evaluated_at_ns - observed_at_ns > maximum_age_ns {
                add_failure(failures, id, FailureReason::ObservationTooOld, required);
            }
        }
    } else if requirement.maximum_data_age_ns.is_some() {
        add_failure(
            failures,
            id,
            FailureReason::ObservationTimeUnknown,
            required,
        );
    }

    if let Some(maximum_interval_ns) = requirement.maximum_sample_interval_ns {
        match component.sample_interval_ns {
            None => add_failure(failures, id, FailureReason::SampleIntervalUnknown, required),
            Some(0) => add_failure(failures, id, FailureReason::SampleIntervalZero, required),
            Some(actual) if actual > maximum_interval_ns => {
                add_failure(failures, id, FailureReason::SampleTooSlow, required);
            }
            Some(_) => {}
        }
    }

    if let Some(maximum_interval_ns) = requirement.maximum_update_interval_ns {
        match component.update_interval_ns {
            None => add_failure(failures, id, FailureReason::UpdateIntervalUnknown, required),
            Some(0) => add_failure(failures, id, FailureReason::UpdateIntervalZero, required),
            Some(actual) if actual > maximum_interval_ns => {
                add_failure(failures, id, FailureReason::UpdateTooSlow, required);
            }
            Some(_) => {}
        }
    }

    if let Some(maximum_latency_ns) = requirement.maximum_p99_latency_ns {
        match component.p99_latency_ns {
            None => add_failure(failures, id, FailureReason::LatencyUnknown, required),
            Some(actual) if actual > maximum_latency_ns => {
                add_failure(failures, id, FailureReason::LatencyExceeded, required);
            }
            Some(_) => {}
        }
    }

    if let Some(maximum_uncertainty_ns) = requirement.maximum_clock_uncertainty_ns {
        match component.clock_mapping_uncertainty_ns {
            None => add_failure(failures, id, FailureReason::ClockMappingUnknown, required),
            Some(actual) if actual > maximum_uncertainty_ns => {
                add_failure(
                    failures,
                    id,
                    FailureReason::ClockUncertaintyExceeded,
                    required,
                );
            }
            Some(_) => {}
        }
    }

    if let Some(expected_frame) = &requirement.expected_coordinate_frame {
        match &component.coordinate_frame {
            None => add_failure(
                failures,
                id,
                FailureReason::CoordinateFrameUnknown,
                required,
            ),
            Some(actual) if actual != expected_frame => {
                add_failure(
                    failures,
                    id,
                    FailureReason::CoordinateFrameMismatch,
                    required,
                );
            }
            Some(_) => {}
        }
    }

    if let Some(maximum_error_um) = requirement.maximum_registration_error_um {
        match component.registration_error_um {
            None => add_failure(
                failures,
                id,
                FailureReason::RegistrationErrorUnknown,
                required,
            ),
            Some(actual) if actual > maximum_error_um => {
                add_failure(
                    failures,
                    id,
                    FailureReason::RegistrationErrorExceeded,
                    required,
                );
            }
            Some(_) => {}
        }
    }

    if let Some(minimum_coverage) = requirement.minimum_coverage_permille {
        match component.coverage_permille {
            None => add_failure(failures, id, FailureReason::CoverageUnknown, required),
            Some(actual) if actual > 1_000 => {
                add_failure(failures, id, FailureReason::CoverageOutOfRange, required);
            }
            Some(actual) if actual < minimum_coverage => {
                add_failure(failures, id, FailureReason::CoverageBelowMinimum, required);
            }
            Some(_) => {}
        }
    }

    if let Some(maximum_occlusion) = requirement.maximum_occlusion_permille {
        match component.occlusion_permille {
            None => add_failure(failures, id, FailureReason::OcclusionUnknown, required),
            Some(actual) if actual > 1_000 => {
                add_failure(failures, id, FailureReason::OcclusionOutOfRange, required);
            }
            Some(actual) if actual > maximum_occlusion => {
                add_failure(failures, id, FailureReason::OcclusionExceeded, required);
            }
            Some(_) => {}
        }
    }

    if let Some(maximum_error) = requirement.maximum_fidelity_error_units {
        match component.fidelity_error_units {
            None => add_failure(failures, id, FailureReason::FidelityUnknown, required),
            Some(actual) if actual > maximum_error => {
                add_failure(failures, id, FailureReason::FidelityExceeded, required);
            }
            Some(_) => {}
        }
    }

    // Different clock domains require an explicit mapping. If a threshold is
    // configured above, the uncertainty is also checked against that bound.
    if component.clock_domain != profile.clock_domain
        && component.clock_mapping_uncertainty_ns.is_none()
        && requirement.maximum_clock_uncertainty_ns.is_none()
    {
        add_failure(failures, id, FailureReason::ClockMappingUnknown, required);
    }
}

fn detect_common_cause_dependencies(
    profile: &TaskProfile,
    components: &[ComponentEvidence],
    failures: &mut Vec<ReadinessFailure>,
) {
    if !profile.independent_sources_required {
        return;
    }

    let required_ids: Vec<&str> = profile
        .requirements
        .iter()
        .filter(|requirement| requirement.required)
        .map(|requirement| requirement.component_id.as_str())
        .collect();

    for (left_index, left_id) in required_ids.iter().enumerate() {
        for right_id in required_ids.iter().skip(left_index + 1) {
            let left = components
                .iter()
                .find(|component| component.component_id.as_str() == *left_id);
            let right = components
                .iter()
                .find(|component| component.component_id.as_str() == *right_id);
            let (Some(left), Some(right)) = (left, right) else {
                continue;
            };
            let shared = left
                .common_cause_domains
                .iter()
                .any(|domain| right.common_cause_domains.contains(domain));
            if shared {
                add_failure(
                    failures,
                    &format!("{left_id}|{right_id}"),
                    FailureReason::CommonCauseDependency,
                    true,
                );
            }
        }
    }
}

/// Evaluate whether a federated source composition is ready for one named task.
///
/// The result is task-scoped and time-bounded. A READY result for inter-pass
/// geometry is not evidence of readiness for fast process control or weld
/// acceptance. Unknown required evidence fails closed.
pub fn evaluate_readiness(
    profile: &TaskProfile,
    components: &[ComponentEvidence],
    evaluated_at_ns: u64,
) -> ReadinessReceipt {
    let mut failures = Vec::new();

    if !nonempty(&profile.task_id)
        || !nonempty(&profile.revision)
        || !nonempty(&profile.clock_domain)
    {
        add_failure(
            &mut failures,
            "__profile__",
            FailureReason::InvalidTaskProfile,
            true,
        );
    }
    if !profile.supported {
        add_failure(
            &mut failures,
            "__profile__",
            FailureReason::UnsupportedTaskProfile,
            true,
        );
    }
    if profile.requirements.is_empty() {
        add_failure(
            &mut failures,
            "__profile__",
            FailureReason::EmptyRequirements,
            true,
        );
    }

    let mut requirement_ids = HashSet::new();
    for requirement in &profile.requirements {
        if !requirement_ids.insert(requirement.component_id.as_str()) {
            add_failure(
                &mut failures,
                &requirement.component_id,
                FailureReason::DuplicateRequirementId,
                true,
            );
        }
    }

    let mut component_ids = HashSet::new();
    for component in components {
        if !component_ids.insert(component.component_id.as_str()) {
            add_failure(
                &mut failures,
                &component.component_id,
                FailureReason::DuplicateComponentIdentity,
                true,
            );
        }
        if !requirement_ids.contains(component.component_id.as_str()) {
            add_failure(
                &mut failures,
                &component.component_id,
                FailureReason::UnboundComponent,
                true,
            );
        }
    }

    for requirement in &profile.requirements {
        let Some(component) = components
            .iter()
            .find(|component| component.component_id == requirement.component_id)
        else {
            let reason = if requirement.required {
                FailureReason::MissingRequiredComponent
            } else {
                FailureReason::MissingOptionalComponent
            };
            add_failure(
                &mut failures,
                &requirement.component_id,
                reason,
                requirement.required,
            );
            continue;
        };
        check_component(
            requirement,
            component,
            profile,
            evaluated_at_ns,
            &mut failures,
        );
    }

    detect_common_cause_dependencies(profile, components, &mut failures);

    failures.sort_by(|left, right| {
        left.component_id
            .cmp(&right.component_id)
            .then_with(|| (left.reason as u8).cmp(&(right.reason as u8)))
    });

    let has_required_failure = failures.iter().any(|failure| failure.required);
    let status = if has_required_failure {
        ReadinessStatus::NotReady
    } else if !failures.is_empty() {
        ReadinessStatus::Degraded
    } else {
        ReadinessStatus::Ready
    };

    let mut evaluated_components: Vec<String> = components
        .iter()
        .map(|component| component.component_id.clone())
        .collect();
    evaluated_components.sort();
    evaluated_components.dedup();

    let fingerprint = readiness_fingerprint(
        &profile.task_id,
        &profile.revision,
        evaluated_at_ns,
        status,
        &evaluated_components,
        &failures,
        components,
    );

    ReadinessReceipt {
        task_id: profile.task_id.clone(),
        profile_revision: profile.revision.clone(),
        evaluated_at_ns,
        status,
        evaluated_components,
        failures,
        fingerprint,
    }
}

fn readiness_fingerprint(
    task_id: &str,
    profile_revision: &str,
    evaluated_at_ns: u64,
    status: ReadinessStatus,
    evaluated_components: &[String],
    failures: &[ReadinessFailure],
    components: &[ComponentEvidence],
) -> u64 {
    // FNV-1a is used only as a stable reproducibility fingerprint. It is not a
    // cryptographic digest, signature, attestation or replacement for SHA-256.
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    };

    feed(task_id.as_bytes());
    feed(profile_revision.as_bytes());
    feed(&evaluated_at_ns.to_le_bytes());
    let status_bytes: &[u8] = match status {
        ReadinessStatus::Ready => b"READY",
        ReadinessStatus::Degraded => b"DEGRADED",
        ReadinessStatus::NotReady => b"NOT_READY",
    };
    feed(status_bytes);

    for component_id in evaluated_components {
        feed(component_id.as_bytes());
    }
    for failure in failures {
        feed(failure.component_id.as_bytes());
        feed(&[failure.reason as u8, u8::from(failure.required)]);
    }

    let mut ordered_components: Vec<&ComponentEvidence> = components.iter().collect();
    ordered_components.sort_by(|left, right| left.component_id.cmp(&right.component_id));
    for component in ordered_components {
        feed(component.component_id.as_bytes());
        feed(component.revision.as_bytes());
        feed(&component.source_sha256);
        feed(&component.transform_receipt_sha256);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 10_000_000_000;

    fn component(
        id: &str,
        owner: &str,
        authority: &str,
        output_scope: &str,
        sample_interval_ns: u64,
        update_interval_ns: u64,
        latency_ns: u64,
    ) -> ComponentEvidence {
        ComponentEvidence {
            component_id: id.to_owned(),
            revision: "R1".to_owned(),
            owner: owner.to_owned(),
            expected_revision: "R1".to_owned(),
            authority: authority.to_owned(),
            output_scope: output_scope.to_owned(),
            source_sha256: [0x11; 32],
            transform_receipt_sha256: [0x22; 32],
            clock_domain: "TwinCAT3_EtherCAT".to_owned(),
            observed_at_ns: Some(NOW - 100_000_000),
            sample_interval_ns: Some(sample_interval_ns),
            update_interval_ns: Some(update_interval_ns),
            p99_latency_ns: Some(latency_ns),
            clock_mapping_uncertainty_ns: Some(1_000),
            coordinate_frame: None,
            registration_error_um: None,
            coverage_permille: None,
            occlusion_permille: None,
            fidelity_error_units: Some(2),
            common_cause_domains: vec!["gateway-A".to_owned()],
        }
    }

    fn requirement(
        id: &str,
        owner: &str,
        authority: &str,
        output_scope: &str,
        required: bool,
    ) -> ComponentRequirement {
        ComponentRequirement {
            component_id: id.to_owned(),
            owner: owner.to_owned(),
            authority: authority.to_owned(),
            output_scope: output_scope.to_owned(),
            required,
            maximum_sample_interval_ns: Some(100_000_000),
            maximum_update_interval_ns: Some(200_000_000),
            maximum_data_age_ns: Some(1_000_000_000),
            maximum_p99_latency_ns: Some(300_000_000),
            maximum_clock_uncertainty_ns: Some(10_000),
            expected_coordinate_frame: None,
            maximum_registration_error_um: None,
            minimum_coverage_permille: None,
            maximum_occlusion_permille: None,
            maximum_fidelity_error_units: Some(10),
        }
    }

    fn interpass_profile() -> TaskProfile {
        let mut scan = requirement(
            "surface-scan",
            "metrology-owner",
            "scan-observation",
            "interpass-surface-geometry",
            true,
        );
        scan.maximum_sample_interval_ns = Some(20_000_000);
        scan.maximum_update_interval_ns = Some(100_000_000);
        scan.maximum_p99_latency_ns = Some(200_000_000);
        scan.expected_coordinate_frame = Some("welding_reference_frame".to_owned());
        scan.maximum_registration_error_um = Some(500);
        scan.minimum_coverage_permille = Some(980);
        scan.maximum_occlusion_permille = Some(20);

        TaskProfile {
            task_id: "INTERPASS_GEOMETRY".to_owned(),
            revision: "PROFILE-R1".to_owned(),
            clock_domain: "TwinCAT3_EtherCAT".to_owned(),
            supported: true,
            independent_sources_required: false,
            requirements: vec![
                scan,
                {
                    let mut requirement = requirement(
                        "robot-tcp",
                        "robot-owner",
                        "robot-observation",
                        "robot-pose",
                        true,
                    );
                    requirement.maximum_sample_interval_ns = Some(8_000_000);
                    requirement.maximum_update_interval_ns = Some(20_000_000);
                    requirement.maximum_p99_latency_ns = Some(50_000_000);
                    requirement
                },
                {
                    let mut requirement = requirement(
                        "process-iv",
                        "weld-power-owner",
                        "process-observation",
                        "voltage-current",
                        true,
                    );
                    requirement.maximum_sample_interval_ns = Some(100_000);
                    requirement.maximum_update_interval_ns = Some(1_000_000);
                    requirement.maximum_p99_latency_ns = Some(2_000_000);
                    requirement
                },
                requirement(
                    "power-source-opcua",
                    "power-source-owner",
                    "equipment-observation",
                    "power-source-state",
                    true,
                ),
            ],
        }
    }

    fn interpass_components() -> Vec<ComponentEvidence> {
        let mut scan = component(
            "surface-scan",
            "metrology-owner",
            "scan-observation",
            "interpass-surface-geometry",
            20_000_000,
            20_000_000,
            50_000_000,
        );
        scan.coordinate_frame = Some("welding_reference_frame".to_owned());
        scan.registration_error_um = Some(100);
        scan.coverage_permille = Some(995);
        scan.occlusion_permille = Some(5);

        vec![
            scan,
            component(
                "robot-tcp",
                "robot-owner",
                "robot-observation",
                "robot-pose",
                8_000_000,
                8_000_000,
                10_000_000,
            ),
            component(
                "process-iv",
                "weld-power-owner",
                "process-observation",
                "voltage-current",
                100_000,
                100_000,
                400_000,
            ),
            component(
                "power-source-opcua",
                "power-source-owner",
                "equipment-observation",
                "power-source-state",
                100_000_000,
                100_000_000,
                150_000_000,
            ),
        ]
    }

    #[test]
    fn interpass_geometry_profile_is_ready_when_all_stream_contracts_pass() {
        let receipt = evaluate_readiness(&interpass_profile(), &interpass_components(), NOW);
        assert_eq!(receipt.status, ReadinessStatus::Ready);
        assert!(receipt.failures.is_empty());
        assert_eq!(receipt.evaluated_components.len(), 4);
    }

    #[test]
    fn the_same_power_source_is_not_ready_for_fast_control() {
        let mut profile = interpass_profile();
        profile.task_id = "FAST_ARC_CONTROL".to_owned();
        profile.revision = "FAST-R1".to_owned();
        let power = profile
            .requirements
            .iter_mut()
            .find(|requirement| requirement.component_id == "power-source-opcua")
            .expect("power-source requirement");
        power.maximum_sample_interval_ns = Some(1_000_000);
        power.maximum_update_interval_ns = Some(1_000_000);
        power.maximum_p99_latency_ns = Some(1_000_000);

        let receipt = evaluate_readiness(&profile, &interpass_components(), NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::SampleTooSlow)
        );
    }

    #[test]
    fn unknown_latency_is_not_treated_as_zero() {
        let mut components = interpass_components();
        components[0].p99_latency_ns = None;
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::LatencyUnknown)
        );
    }

    #[test]
    fn stale_observation_causes_not_ready() {
        let mut components = interpass_components();
        components[0].observed_at_ns = Some(NOW - 2_000_000_000);
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::ObservationTooOld)
        );
    }

    #[test]
    fn future_observation_causes_not_ready() {
        let mut components = interpass_components();
        components[0].observed_at_ns = Some(NOW + 1);
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::ObservationFromFuture)
        );
    }

    #[test]
    fn unknown_clock_mapping_causes_not_ready() {
        let mut components = interpass_components();
        components[0].clock_mapping_uncertainty_ns = None;
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::ClockMappingUnknown)
        );
    }

    #[test]
    fn excessive_clock_uncertainty_causes_not_ready() {
        let mut components = interpass_components();
        components[0].clock_mapping_uncertainty_ns = Some(20_000);
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::ClockUncertaintyExceeded)
        );
    }

    #[test]
    fn wrong_coordinate_frame_causes_not_ready() {
        let mut components = interpass_components();
        components[0].coordinate_frame = Some("robot_base".to_owned());
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::CoordinateFrameMismatch)
        );
    }

    #[test]
    fn insufficient_scan_coverage_causes_not_ready() {
        let mut components = interpass_components();
        components[0].coverage_permille = Some(900);
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::CoverageBelowMinimum)
        );
    }

    #[test]
    fn excessive_scan_occlusion_causes_not_ready() {
        let mut components = interpass_components();
        components[0].occlusion_permille = Some(100);
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::OcclusionExceeded)
        );
    }

    #[test]
    fn stale_transform_evidence_causes_not_ready() {
        let mut components = interpass_components();
        components[0].transform_receipt_sha256 = [0; 32];
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::MissingTransformReceipt)
        );
    }

    #[test]
    fn component_revision_mismatch_causes_not_ready() {
        let mut components = interpass_components();
        components[0].revision = "R2".to_owned();
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::ComponentRevisionMismatch)
        );
    }

    #[test]
    fn authority_mismatch_causes_not_ready() {
        let mut components = interpass_components();
        components[0].authority = "planner-asserted".to_owned();
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::AuthorityMismatch)
        );
    }

    #[test]
    fn optional_component_failure_degrades_but_does_not_mark_not_ready() {
        let mut profile = interpass_profile();
        let mut optional = requirement(
            "optional-video",
            "video-owner",
            "video-observation",
            "weld-pool-video",
            false,
        );
        optional.maximum_sample_interval_ns = Some(25_000_000);
        optional.maximum_update_interval_ns = Some(50_000_000);
        optional.maximum_p99_latency_ns = Some(200_000_000);
        profile.requirements.push(optional);
        let receipt = evaluate_readiness(&profile, &interpass_components(), NOW);
        assert_eq!(receipt.status, ReadinessStatus::Degraded);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::MissingOptionalComponent)
        );
    }

    #[test]
    fn missing_required_component_is_not_ready() {
        let receipt = evaluate_readiness(&interpass_profile(), &[], NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert_eq!(
            receipt
                .failures
                .iter()
                .filter(|failure| failure.reason == FailureReason::MissingRequiredComponent)
                .count(),
            4
        );
    }

    #[test]
    fn shared_common_cause_is_rejected_when_independence_is_required() {
        let mut profile = interpass_profile();
        profile.independent_sources_required = true;
        let receipt = evaluate_readiness(&profile, &interpass_components(), NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::CommonCauseDependency)
        );
    }

    #[test]
    fn unlisted_component_is_not_silently_composed() {
        let mut components = interpass_components();
        components.push(component(
            "unlisted-camera",
            "camera-owner",
            "camera-observation",
            "video",
            25_000_000,
            25_000_000,
            80_000_000,
        ));
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::UnboundComponent)
        );
    }

    #[test]
    fn deterministic_fingerprint_reproduces_for_identical_inputs() {
        let profile = interpass_profile();
        let components = interpass_components();
        let first = evaluate_readiness(&profile, &components, NOW);
        let second = evaluate_readiness(&profile, &components, NOW);
        assert_eq!(first, second);
        assert_ne!(first.fingerprint, 0);
    }

    #[test]
    fn profile_revision_change_changes_receipt_fingerprint() {
        let components = interpass_components();
        let first_profile = interpass_profile();
        let mut second_profile = interpass_profile();
        second_profile.revision = "PROFILE-R2".to_owned();
        let first = evaluate_readiness(&first_profile, &components, NOW);
        let second = evaluate_readiness(&second_profile, &components, NOW);
        assert_ne!(first.fingerprint, second.fingerprint);
    }

    #[test]
    fn unsupported_quality_task_does_not_become_ready() {
        let mut profile = interpass_profile();
        profile.task_id = "POST_WELD_NDT_ACCEPTANCE".to_owned();
        profile.supported = false;
        let receipt = evaluate_readiness(&profile, &interpass_components(), NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::UnsupportedTaskProfile)
        );
    }

    #[test]
    fn duplicate_component_id_causes_not_ready() {
        let mut components = interpass_components();
        components.push(components[0].clone());
        let receipt = evaluate_readiness(&interpass_profile(), &components, NOW);
        assert_eq!(receipt.status, ReadinessStatus::NotReady);
        assert!(
            receipt
                .failures
                .iter()
                .any(|failure| failure.reason == FailureReason::DuplicateComponentIdentity)
        );
    }
}
