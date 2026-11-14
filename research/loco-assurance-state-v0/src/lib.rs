//! Deterministic certificate-validity checks for locomotion assurance.
//!
//! Physical viability and assurance validity are separate gates. Invalid or
//! incomplete evidence does not prove the robot physically unsafe; it means
//! the certificate cannot currently justify the proposed action.
//!
//! This reference does not validate physical limits, sensor calibration,
//! clocks, thermal models, actuator dynamics or the viability certificate.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuranceStatus {
    Valid,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssuranceFinding {
    InvalidCertificateShape,
    CertificateNotYetValid,
    CertificateExpired,
    ModelDigestMismatch,
    MissingSensorEvidence(String),
    SensorCalibrationMismatch(String),
    SensorUnhealthy(String),
    SensorOutOfDistribution(String),
    SensorTimestampMissing(String),
    SensorTimestampInFuture(String),
    SensorStale(String),
    ClockUncertain,
    ComputeStartInFuture,
    ComputeCompletionMissing,
    ComputeCompletionInFuture,
    ComputeTimeReversed,
    ComputeDeadlineMissed,
    ComputeLatencyExceeded,
    CommandAgeExceeded,
    ActuatorOverTemperature,
    SupplyVoltageBelowMinimum,
    ActuatorTrackingErrorExceeded,
    ActuatorDerated,
}

impl AssuranceFinding {
    fn is_unknown(&self) -> bool {
        matches!(
            self,
            Self::MissingSensorEvidence(_)
                | Self::SensorTimestampMissing(_)
                | Self::SensorTimestampInFuture(_)
                | Self::ClockUncertain
                | Self::ComputeStartInFuture
                | Self::ComputeCompletionMissing
                | Self::ComputeCompletionInFuture
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertificateEnvelope {
    pub certificate_id: String,
    pub model_digest: String,
    pub issued_at_ns: u64,
    /// Inclusive expiry boundary.
    pub valid_until_ns: u64,
    pub required_sensor_calibrations: BTreeMap<String, String>,
    pub maximum_sensor_age_ns: u64,
    pub maximum_actuator_temperature_millicelsius: i32,
    pub minimum_supply_voltage_mv: u32,
    pub maximum_tracking_error_microrad: u64,
    pub maximum_compute_latency_ns: u64,
    pub maximum_command_age_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorEvidence {
    pub sensor_id: String,
    pub calibration_digest: String,
    pub observed_at_ns: Option<u64>,
    pub healthy: bool,
    pub out_of_distribution: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActuatorEvidence {
    pub temperature_millicelsius: i32,
    pub supply_voltage_mv: u32,
    pub tracking_error_microrad: u64,
    pub derated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEvidence {
    /// Gate-evaluation time immediately before action application.
    pub now_ns: u64,
    pub model_digest: String,
    pub sensors: BTreeMap<String, SensorEvidence>,
    pub actuator: ActuatorEvidence,
    pub compute_started_at_ns: u64,
    pub compute_completed_at_ns: Option<u64>,
    pub action_deadline_at_ns: u64,
    pub clock_uncertain: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssuranceReport {
    pub status: AssuranceStatus,
    /// Stable sorted and deduplicated findings for deterministic receipts.
    pub findings: Vec<AssuranceFinding>,
}

impl AssuranceReport {
    fn from_findings(mut findings: Vec<AssuranceFinding>) -> Self {
        findings.sort();
        findings.dedup();
        let status = if findings.iter().any(|finding| !finding.is_unknown()) {
            AssuranceStatus::Invalid
        } else if findings.is_empty() {
            AssuranceStatus::Valid
        } else {
            AssuranceStatus::Unknown
        };
        Self { status, findings }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalDecision {
    PermitCertifiedAction,
    ActionRejectedByPhysicalGate,
    RevalidationRequired,
    EvidenceUnknown,
}

impl OperationalDecision {
    pub fn permits_action(self) -> bool {
        matches!(self, Self::PermitCertifiedAction)
    }

    pub fn requires_fallback_or_revalidation(self) -> bool {
        !self.permits_action()
    }
}

/// Evaluate the evidence envelope at the instant the action is to be applied.
/// Re-run this gate if the command is delayed after the snapshot.
pub fn evaluate_assurance(
    certificate: &CertificateEnvelope,
    runtime: &RuntimeEvidence,
) -> AssuranceReport {
    let mut findings = Vec::new();

    if certificate.certificate_id.trim().is_empty()
        || certificate.model_digest.trim().is_empty()
        || certificate.issued_at_ns > certificate.valid_until_ns
        || certificate
            .required_sensor_calibrations
            .iter()
            .any(|(id, digest)| id.trim().is_empty() || digest.trim().is_empty())
    {
        findings.push(AssuranceFinding::InvalidCertificateShape);
    }

    let clock_known = !runtime.clock_uncertain;
    if !clock_known {
        findings.push(AssuranceFinding::ClockUncertain);
    } else {
        if runtime.now_ns < certificate.issued_at_ns {
            findings.push(AssuranceFinding::CertificateNotYetValid);
        }
        if runtime.now_ns > certificate.valid_until_ns {
            findings.push(AssuranceFinding::CertificateExpired);
        }
    }
    if runtime.model_digest != certificate.model_digest {
        findings.push(AssuranceFinding::ModelDigestMismatch);
    }

    for (sensor_id, required_calibration) in &certificate.required_sensor_calibrations {
        let Some(sensor) = runtime.sensors.get(sensor_id) else {
            findings.push(AssuranceFinding::MissingSensorEvidence(sensor_id.clone()));
            continue;
        };

        if sensor.sensor_id != *sensor_id || sensor.calibration_digest != *required_calibration {
            findings.push(AssuranceFinding::SensorCalibrationMismatch(
                sensor_id.clone(),
            ));
        }
        if !sensor.healthy {
            findings.push(AssuranceFinding::SensorUnhealthy(sensor_id.clone()));
        }
        if sensor.out_of_distribution {
            findings.push(AssuranceFinding::SensorOutOfDistribution(sensor_id.clone()));
        }

        if clock_known {
            match sensor.observed_at_ns {
                None => findings.push(AssuranceFinding::SensorTimestampMissing(sensor_id.clone())),
                Some(timestamp) if timestamp > runtime.now_ns => {
                    findings.push(AssuranceFinding::SensorTimestampInFuture(sensor_id.clone()));
                }
                Some(timestamp)
                    if runtime.now_ns - timestamp > certificate.maximum_sensor_age_ns =>
                {
                    findings.push(AssuranceFinding::SensorStale(sensor_id.clone()));
                }
                Some(_) => {}
            }
        } else if sensor.observed_at_ns.is_none() {
            findings.push(AssuranceFinding::SensorTimestampMissing(sensor_id.clone()));
        }
    }

    if runtime.actuator.temperature_millicelsius
        > certificate.maximum_actuator_temperature_millicelsius
    {
        findings.push(AssuranceFinding::ActuatorOverTemperature);
    }
    if runtime.actuator.supply_voltage_mv < certificate.minimum_supply_voltage_mv {
        findings.push(AssuranceFinding::SupplyVoltageBelowMinimum);
    }
    if runtime.actuator.tracking_error_microrad > certificate.maximum_tracking_error_microrad {
        findings.push(AssuranceFinding::ActuatorTrackingErrorExceeded);
    }
    if runtime.actuator.derated {
        findings.push(AssuranceFinding::ActuatorDerated);
    }

    if clock_known {
        if runtime.compute_started_at_ns > runtime.now_ns {
            findings.push(AssuranceFinding::ComputeStartInFuture);
        }
        if runtime.now_ns > runtime.action_deadline_at_ns {
            findings.push(AssuranceFinding::ComputeDeadlineMissed);
        }

        match runtime.compute_completed_at_ns {
            None => findings.push(AssuranceFinding::ComputeCompletionMissing),
            Some(completed_at) if completed_at > runtime.now_ns => {
                findings.push(AssuranceFinding::ComputeCompletionInFuture);
                if completed_at > runtime.action_deadline_at_ns {
                    findings.push(AssuranceFinding::ComputeDeadlineMissed);
                }
            }
            Some(completed_at) if completed_at < runtime.compute_started_at_ns => {
                findings.push(AssuranceFinding::ComputeTimeReversed);
            }
            Some(completed_at) => {
                if completed_at > runtime.action_deadline_at_ns {
                    findings.push(AssuranceFinding::ComputeDeadlineMissed);
                }
                if completed_at - runtime.compute_started_at_ns
                    > certificate.maximum_compute_latency_ns
                {
                    findings.push(AssuranceFinding::ComputeLatencyExceeded);
                }
                if runtime.now_ns - completed_at > certificate.maximum_command_age_ns {
                    findings.push(AssuranceFinding::CommandAgeExceeded);
                }
            }
        }
    } else if runtime.compute_completed_at_ns.is_none() {
        findings.push(AssuranceFinding::ComputeCompletionMissing);
    }

    AssuranceReport::from_findings(findings)
}

/// Combine independent physical admissibility with evidence validity.
///
/// The physical gate has priority. No evidence status can override a rejected
/// action, and a valid evidence report cannot create physical viability.
pub fn operational_decision(
    robustly_viable_action: bool,
    assurance: &AssuranceReport,
) -> OperationalDecision {
    if !robustly_viable_action {
        return OperationalDecision::ActionRejectedByPhysicalGate;
    }
    match assurance.status {
        AssuranceStatus::Valid => OperationalDecision::PermitCertifiedAction,
        AssuranceStatus::Unknown => OperationalDecision::EvidenceUnknown,
        AssuranceStatus::Invalid => OperationalDecision::RevalidationRequired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn certificate() -> CertificateEnvelope {
        CertificateEnvelope {
            certificate_id: "cert-hex-01".to_owned(),
            model_digest: "model-a".to_owned(),
            issued_at_ns: 90,
            valid_until_ns: 110,
            required_sensor_calibrations: BTreeMap::from([
                ("contact-left".to_owned(), "cal-contact-v2".to_owned()),
                ("imu".to_owned(), "cal-imu-v3".to_owned()),
            ]),
            maximum_sensor_age_ns: 20,
            maximum_actuator_temperature_millicelsius: 55_000,
            minimum_supply_voltage_mv: 4_500,
            maximum_tracking_error_microrad: 500,
            maximum_compute_latency_ns: 20,
            maximum_command_age_ns: 10,
        }
    }

    fn sensor(id: &str, calibration: &str, observed_at_ns: Option<u64>) -> SensorEvidence {
        SensorEvidence {
            sensor_id: id.to_owned(),
            calibration_digest: calibration.to_owned(),
            observed_at_ns,
            healthy: true,
            out_of_distribution: false,
        }
    }

    fn runtime() -> RuntimeEvidence {
        RuntimeEvidence {
            now_ns: 100,
            model_digest: "model-a".to_owned(),
            sensors: BTreeMap::from([
                (
                    "contact-left".to_owned(),
                    sensor("contact-left", "cal-contact-v2", Some(95)),
                ),
                ("imu".to_owned(), sensor("imu", "cal-imu-v3", Some(96))),
            ]),
            actuator: ActuatorEvidence {
                temperature_millicelsius: 40_000,
                supply_voltage_mv: 5_000,
                tracking_error_microrad: 100,
                derated: false,
            },
            compute_started_at_ns: 80,
            compute_completed_at_ns: Some(95),
            action_deadline_at_ns: 105,
            clock_uncertain: false,
        }
    }

    #[test]
    fn fresh_evidence_and_viable_action_are_permitted() {
        let report = evaluate_assurance(&certificate(), &runtime());
        assert_eq!(report.status, AssuranceStatus::Valid);
        assert!(report.findings.is_empty());
        assert_eq!(
            operational_decision(true, &report),
            OperationalDecision::PermitCertifiedAction
        );
    }

    #[test]
    fn valid_certificate_cannot_override_non_viable_action() {
        let report = evaluate_assurance(&certificate(), &runtime());
        assert_eq!(
            operational_decision(false, &report),
            OperationalDecision::ActionRejectedByPhysicalGate
        );
    }

    #[test]
    fn validity_interval_includes_exact_issue_and_expiry_boundaries() {
        let cert = certificate();
        let mut at_issue = runtime();
        at_issue.now_ns = cert.issued_at_ns;
        at_issue.action_deadline_at_ns = 110;
        at_issue.compute_completed_at_ns = Some(90);
        at_issue.compute_started_at_ns = 90;
        for evidence in at_issue.sensors.values_mut() {
            evidence.observed_at_ns = Some(90);
        }
        assert_eq!(
            evaluate_assurance(&cert, &at_issue).status,
            AssuranceStatus::Valid
        );

        let mut at_expiry = at_issue;
        at_expiry.now_ns = cert.valid_until_ns;
        at_expiry.action_deadline_at_ns = 120;
        at_expiry.compute_started_at_ns = 100;
        at_expiry.compute_completed_at_ns = Some(110);
        for evidence in at_expiry.sensors.values_mut() {
            evidence.observed_at_ns = Some(110);
        }
        assert_eq!(
            evaluate_assurance(&cert, &at_expiry).status,
            AssuranceStatus::Valid
        );
    }

    #[test]
    fn certificate_before_issue_or_after_expiry_is_invalid() {
        let cert = certificate();
        let mut before = runtime();
        before.now_ns = 89;
        let before_report = evaluate_assurance(&cert, &before);
        assert_eq!(before_report.status, AssuranceStatus::Invalid);
        assert!(
            before_report
                .findings
                .contains(&AssuranceFinding::CertificateNotYetValid)
        );

        let mut after = runtime();
        after.now_ns = 111;
        let after_report = evaluate_assurance(&cert, &after);
        assert_eq!(after_report.status, AssuranceStatus::Invalid);
        assert!(
            after_report
                .findings
                .contains(&AssuranceFinding::CertificateExpired)
        );
    }

    #[test]
    fn model_digest_mismatch_invalidates_the_certificate() {
        let mut current = runtime();
        current.model_digest = "model-b".to_owned();
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Invalid);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::ModelDigestMismatch)
        );
    }

    #[test]
    fn missing_required_sensor_is_unknown_not_a_physical_unsafe_claim() {
        let mut current = runtime();
        current.sensors.remove("contact-left");
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Unknown);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::MissingSensorEvidence(
                    "contact-left".to_owned()
                ))
        );
        assert_eq!(
            operational_decision(true, &report),
            OperationalDecision::EvidenceUnknown
        );
    }

    #[test]
    fn calibration_mismatch_invalidates_even_healthy_fresh_sensor() {
        let mut current = runtime();
        current.sensors.get_mut("imu").unwrap().calibration_digest = "old-calibration".to_owned();
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Invalid);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::SensorCalibrationMismatch(
                    "imu".to_owned()
                ))
        );
    }

    #[test]
    fn unhealthy_stale_and_out_of_distribution_sensors_invalidate() {
        let mut current = runtime();
        let contact = current.sensors.get_mut("contact-left").unwrap();
        contact.healthy = false;
        contact.out_of_distribution = true;
        contact.observed_at_ns = Some(70);
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Invalid);
        assert!(report.findings.contains(&AssuranceFinding::SensorUnhealthy(
            "contact-left".to_owned()
        )));
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::SensorOutOfDistribution(
                    "contact-left".to_owned()
                ))
        );
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::SensorStale("contact-left".to_owned()))
        );
    }

    #[test]
    fn missing_or_future_sensor_timestamp_is_unknown() {
        let mut missing = runtime();
        missing.sensors.get_mut("imu").unwrap().observed_at_ns = None;
        assert_eq!(
            evaluate_assurance(&certificate(), &missing).status,
            AssuranceStatus::Unknown
        );

        let mut future = runtime();
        future.sensors.get_mut("imu").unwrap().observed_at_ns = Some(101);
        let report = evaluate_assurance(&certificate(), &future);
        assert_eq!(report.status, AssuranceStatus::Unknown);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::SensorTimestampInFuture("imu".to_owned()))
        );
    }

    #[test]
    fn actuator_temperature_voltage_tracking_and_derating_are_checked() {
        let mut current = runtime();
        current.actuator.temperature_millicelsius = 55_001;
        current.actuator.supply_voltage_mv = 4_499;
        current.actuator.tracking_error_microrad = 501;
        current.actuator.derated = true;
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Invalid);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::ActuatorOverTemperature)
        );
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::SupplyVoltageBelowMinimum)
        );
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::ActuatorTrackingErrorExceeded)
        );
        assert!(report.findings.contains(&AssuranceFinding::ActuatorDerated));
    }

    #[test]
    fn exact_actuator_envelope_boundaries_are_permitted() {
        let mut current = runtime();
        current.actuator.temperature_millicelsius = 55_000;
        current.actuator.supply_voltage_mv = 4_500;
        current.actuator.tracking_error_microrad = 500;
        assert_eq!(
            evaluate_assurance(&certificate(), &current).status,
            AssuranceStatus::Valid
        );
    }

    #[test]
    fn missed_deadline_invalidates_certificate_use() {
        let mut current = runtime();
        current.action_deadline_at_ns = 99;
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Invalid);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::ComputeDeadlineMissed)
        );
    }

    #[test]
    fn compute_latency_and_command_age_are_independent_limits() {
        let mut slow_compute = runtime();
        slow_compute.compute_started_at_ns = 70;
        let report = evaluate_assurance(&certificate(), &slow_compute);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::ComputeLatencyExceeded)
        );

        let mut stale_command = runtime();
        stale_command.compute_completed_at_ns = Some(80);
        let report = evaluate_assurance(&certificate(), &stale_command);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::CommandAgeExceeded)
        );
    }

    #[test]
    fn missing_or_future_compute_completion_is_unknown() {
        let mut missing = runtime();
        missing.compute_completed_at_ns = None;
        assert_eq!(
            evaluate_assurance(&certificate(), &missing).status,
            AssuranceStatus::Unknown
        );

        let mut future = runtime();
        future.compute_completed_at_ns = Some(101);
        assert_eq!(
            evaluate_assurance(&certificate(), &future).status,
            AssuranceStatus::Unknown
        );
    }

    #[test]
    fn clock_uncertainty_does_not_use_untrusted_time_to_expire_evidence() {
        let mut current = runtime();
        current.now_ns = 1_000;
        current.clock_uncertain = true;
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Unknown);
        assert!(
            !report
                .findings
                .contains(&AssuranceFinding::CertificateExpired)
        );
        assert!(
            !report
                .findings
                .contains(&AssuranceFinding::SensorStale("imu".to_owned()))
        );
        assert!(report.findings.contains(&AssuranceFinding::ClockUncertain));
    }

    #[test]
    fn clock_uncertainty_does_not_hide_independent_actuator_failure() {
        let mut current = runtime();
        current.clock_uncertain = true;
        current.actuator.derated = true;
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Invalid);
        assert!(report.findings.contains(&AssuranceFinding::ClockUncertain));
        assert!(report.findings.contains(&AssuranceFinding::ActuatorDerated));
    }

    #[test]
    fn clock_uncertainty_is_unknown_even_when_other_evidence_is_good() {
        let mut current = runtime();
        current.clock_uncertain = true;
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(report.status, AssuranceStatus::Unknown);
        assert!(report.findings.contains(&AssuranceFinding::ClockUncertain));
    }

    #[test]
    fn multiple_findings_are_sorted_and_deduplicated_deterministically() {
        let mut current = runtime();
        current.model_digest = "wrong".to_owned();
        current.actuator.derated = true;
        current.actuator.temperature_millicelsius = 60_000;
        current.sensors.get_mut("imu").unwrap().healthy = false;
        let first = evaluate_assurance(&certificate(), &current);
        let second = evaluate_assurance(&certificate(), &current);
        assert_eq!(first, second);
        let mut sorted = first.findings.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(first.findings, sorted);
    }

    #[test]
    fn invalid_evidence_requires_revalidation_not_physical_unsafe_label() {
        let mut current = runtime();
        current.sensors.get_mut("contact-left").unwrap().healthy = false;
        let report = evaluate_assurance(&certificate(), &current);
        assert_eq!(
            operational_decision(true, &report),
            OperationalDecision::RevalidationRequired
        );
        assert!(OperationalDecision::RevalidationRequired.requires_fallback_or_revalidation());
    }

    #[test]
    fn malformed_certificate_shape_is_invalid() {
        let mut cert = certificate();
        cert.certificate_id.clear();
        cert.valid_until_ns = cert.issued_at_ns - 1;
        let report = evaluate_assurance(&cert, &runtime());
        assert_eq!(report.status, AssuranceStatus::Invalid);
        assert!(
            report
                .findings
                .contains(&AssuranceFinding::InvalidCertificateShape)
        );
    }
}
