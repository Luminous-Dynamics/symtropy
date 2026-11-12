//! JSON CLI for paired degradation/recovery campaign auditing.
//!
//! Usage:
//!   degradation-recovery-envelope <campaign.json> <report.json>
//!
//! Exit codes: 0 valid, 1 rejected campaign (report is written), 2 input error.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use degradation_recovery_envelope::{
    AuditReport, Campaign, audit_campaign, severity_sweep_schedule, standard_fault_schedule,
};
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

fn audit_value(input: &str) -> Result<AuditReport, String> {
    let campaign: Campaign = serde_json::from_str(input)
        .map_err(|error| format!("campaign JSON does not match the typed schema: {error}"))?;
    Ok(audit_campaign(&campaign))
}

fn write_report(path: &Path, report: &AuditReport) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(report)?)?;
    Ok(())
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args
        .first()
        .is_some_and(|value| value == "generate-severity-sweep")
    {
        if args.len() != 9 {
            return Err(
                "usage: degradation-recovery-envelope generate-severity-sweep <first-start-tick> <spacing-ticks> <exposure-ticks> <detection-deadline-ticks> <containment-deadline-ticks> <recovery-quorum-ticks> <severity-ppm-csv> <output.json>"
                    .into(),
            );
        }
        let parse = |index: usize, name: &str| -> Result<u64, Box<dyn Error>> {
            args[index]
                .parse::<u64>()
                .map_err(|error| format!("invalid {name}: {error}").into())
        };
        let severities = args[7]
            .split(',')
            .map(|value| {
                value
                    .trim()
                    .parse::<u32>()
                    .map_err(|error| format!("invalid severity {value:?}: {error}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("cannot parse severity sweep: {error}"))?;
        let events = severity_sweep_schedule(
            parse(1, "first-start-tick")?,
            parse(2, "spacing-ticks")?,
            parse(3, "exposure-ticks")?,
            parse(4, "detection-deadline-ticks")?,
            parse(5, "containment-deadline-ticks")?,
            parse(6, "recovery-quorum-ticks")?,
            &severities,
        )
        .map_err(|error| format!("cannot generate severity sweep: {error}"))?;
        let output_path = Path::new(&args[8]);
        if let Some(parent) = output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, serde_json::to_vec_pretty(&events)?)?;
        println!(
            "generated {} isolated fault/severity events; schedule: {}",
            events.len(),
            output_path.display()
        );
        return Ok(0);
    }
    if args
        .first()
        .is_some_and(|value| value == "generate-schedule")
    {
        if args.len() != 8 {
            return Err(
                "usage: degradation-recovery-envelope generate-schedule <first-start-tick> <spacing-ticks> <exposure-ticks> <detection-deadline-ticks> <containment-deadline-ticks> <recovery-quorum-ticks> <output.json>"
                    .into(),
            );
        }
        let parse = |index: usize, name: &str| -> Result<u64, Box<dyn Error>> {
            args[index]
                .parse::<u64>()
                .map_err(|error| format!("invalid {name}: {error}").into())
        };
        let events = standard_fault_schedule(
            parse(1, "first-start-tick")?,
            parse(2, "spacing-ticks")?,
            parse(3, "exposure-ticks")?,
            parse(4, "detection-deadline-ticks")?,
            parse(5, "containment-deadline-ticks")?,
            parse(6, "recovery-quorum-ticks")?,
        )
        .map_err(|error| format!("cannot generate schedule: {error}"))?;
        let output_path = Path::new(&args[7]);
        if let Some(parent) = output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, serde_json::to_vec_pretty(&events)?)?;
        println!(
            "generated {} deterministic fault events; schedule: {}",
            events.len(),
            output_path.display()
        );
        return Ok(0);
    }
    if args.len() != 2 {
        return Err(
            "usage: degradation-recovery-envelope <campaign.json> <report.json>\n       degradation-recovery-envelope generate-schedule <first-start-tick> <spacing-ticks> <exposure-ticks> <detection-deadline-ticks> <containment-deadline-ticks> <recovery-quorum-ticks> <output.json>\n       degradation-recovery-envelope generate-severity-sweep <first-start-tick> <spacing-ticks> <exposure-ticks> <detection-deadline-ticks> <containment-deadline-ticks> <recovery-quorum-ticks> <severity-ppm-csv> <output.json>".into(),
        );
    }
    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("campaign input must be a regular file no larger than 64 MiB".into());
    }
    let input = fs::read_to_string(input_path)?;
    let report =
        audit_value(&input).map_err(|error| format!("campaign input rejected: {error}"))?;
    write_report(output_path, &report)?;
    println!(
        "degradation/recovery audit: {} ({} findings); report: {}",
        report.disposition,
        report.findings.len(),
        output_path.display()
    );
    Ok(if report.disposition == "valid" { 0 } else { 1 })
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("degradation-recovery-envelope: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_camera_drop_fixture_is_valid() {
        let report = audit_value(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        assert_eq!(report.disposition, "valid");
        assert!(report.findings.is_empty());
        assert_eq!(report.expected_ticks, 8);
        assert_eq!(report.events[0].recovery_tick, Some(8));
    }

    #[test]
    fn compound_common_cause_fixture_audits_two_faults_separately() {
        let report = audit_value(include_str!(
            "../fixtures/compound-common-cause-camera-clock-v1.json"
        ))
        .unwrap();
        assert_eq!(report.disposition, "valid");
        assert_eq!(report.events.len(), 2);
        assert!(report.events.iter().all(|event| {
            event.causality == degradation_recovery_envelope::FaultCausality::CommonCause
                && event.causal_group_id.as_deref() == Some("shared-power-clock-domain-001")
        }));
        assert!(report.events.iter().all(|event| {
            event.detection_latency_ticks == Some(0) && event.containment_latency_ticks == Some(0)
        }));
    }

    #[test]
    fn cli_report_retains_each_metric_axis_separately() {
        let report = audit_value(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        let rendered = serde_json::to_value(report).unwrap();
        assert!(rendered.get("clean_control_decision_latency").is_some());
        assert!(rendered.get("faulted_decision_latency").is_some());
        assert!(rendered.get("clean_control_inference_energy").is_some());
        assert!(rendered.get("faulted_inference_energy").is_some());
        assert!(rendered.get("composite_score").is_none());
    }

    #[test]
    fn checked_in_severity_sweep_fixture_matches_generator_exactly() {
        let generated = severity_sweep_schedule(
            10,
            5,
            3,
            1,
            1,
            2,
            &[100_000, 250_000, 500_000, 750_000, 1_000_000],
        )
        .unwrap();
        let fixture: Vec<degradation_recovery_envelope::FaultEvent> =
            serde_json::from_str(include_str!("../fixtures/severity-sweep.json")).unwrap();
        assert_eq!(fixture, generated);
        assert_eq!(fixture.len(), 35);
    }

    #[test]
    fn cli_exposes_severity_sweep_generator() {
        let events =
            severity_sweep_schedule(10, 5, 3, 1, 1, 2, &[100_000, 500_000, 1_000_000]).unwrap();
        assert_eq!(events.len(), 21);
        assert_eq!(events[0].severity_ppm, 100_000);
        assert_eq!(events[2].severity_ppm, 1_000_000);
    }

    #[test]
    fn cli_exposes_deterministic_seven_family_fault_schedule() {
        let events = standard_fault_schedule(10, 5, 3, 1, 1, 2).unwrap();
        let rendered = serde_json::to_value(events).unwrap();
        assert_eq!(rendered.as_array().unwrap().len(), 7);
        assert_eq!(rendered[0]["kind"], "camera_drop");
        assert_eq!(rendered[6]["kind"], "compute_overrun");
    }

    #[test]
    fn campaign_and_report_schemas_are_valid_json_documents() {
        let campaign_schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/campaign-v1.schema.json")).unwrap();
        let report_schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/report-v1.schema.json")).unwrap();
        assert_eq!(
            campaign_schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            report_schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        for field in [
            "model_id",
            "model_status",
            "checkpoint_sha256",
            "hardware_id",
            "inference_precision",
            "power_profile",
            "energy_measurement_method",
            "policy_visible_inputs",
            "fault_schedule_visible_to_policy",
        ] {
            assert!(
                campaign_schema["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| entry == field)
            );
        }
        for field in [
            "clean_control_deadline_miss_ticks",
            "model_id",
            "checkpoint_sha256",
            "hardware_id",
            "policy_visible_inputs",
            "clean_control_disposition",
            "faulted_run_disposition",
        ] {
            assert!(
                report_schema["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| entry == field)
            );
        }
        assert!(
            campaign_schema["$defs"]["tick"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "fault_detection_event_ids")
        );
        for field in ["causality", "causal_group_id", "parent_event_id"] {
            assert!(
                campaign_schema["$defs"]["event"]["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| entry == field)
            );
            assert!(
                report_schema["$defs"]["event_metrics"]["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|entry| entry == field)
            );
        }
        assert!(
            report_schema["$defs"]["event_metrics"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "severity_ppm")
        );
    }

    #[test]
    fn cli_requires_model_and_runtime_provenance_fields() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        value.as_object_mut().unwrap().remove("model_id");
        assert!(audit_value(&value.to_string()).is_err());
    }
}

#[cfg(test)]
mod disposition_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn cli_keeps_incomplete_and_censored_run_dispositions_visible() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        value["clean_control"]["disposition"] = json!("incomplete");
        value["faulted_run"]["disposition"] = json!("censored");
        let report = audit_value(&value.to_string()).unwrap();
        assert_eq!(
            report.clean_control_disposition,
            degradation_recovery_envelope::RunDisposition::Incomplete
        );
        assert_eq!(
            report.faulted_run_disposition,
            degradation_recovery_envelope::RunDisposition::Censored
        );
        assert_eq!(report.disposition, "rejected");
    }
}
