//! Export synthetic waveform replay as evidence-typed plasma observation frames.
//!
//! This adapter is intentionally one-way and synthetic-only. It never emits
//! Measured, Delayed, or Degraded measurement dispositions because replay input
//! has no physical source or calibration attestation. Numeric values from
//! synthetic nominal, delayed, or degraded samples are exported as Synthetic.
//! Value-less dropout and invalid samples remain Unavailable or Invalid.
//! The envelope retains the exact replay state and fault identity.

use super::ReplayError;
use super::digest_serialized;
use super::waveform::{
    WAVEFORM_CLAIM_CEILING, WAVEFORM_EVIDENCE_CLASS, WaveformReceipt, WaveformSampleState,
    verify_waveform_receipt,
};
use serde::{Deserialize, Serialize};

pub const OBSERVATION_EXPORT_SCHEMA: &str = "luminous.plasma-synthetic-observation-export.v0.1";
pub const PLASMA_OBSERVATION_FRAME_SCHEMA: &str = "luminous.plasma-observation-frame.v0.1";
pub const SYNTHETIC_REPLAY_MODEL_ID: &str = "symtropy-plasma-waveform-fault-replay";
pub const SYNTHETIC_REPLAY_MODEL_REVISION: &str = "0.1.0";
pub const SYNTHETIC_REPLAY_DIAGNOSTIC_SYSTEM_ID: &str = "symtropy.synthetic-waveform-replay";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum ExportObservationDisposition {
    Synthetic {
        value: f64,
        units: String,
        model_id: String,
        model_revision: String,
        input_digest: String,
    },
    Unavailable {
        reason: String,
    },
    Invalid {
        reason: String,
    },
}

/// Structural wire-compatible representation of Symthaea PlasmaObservationFrame
/// v0.1. This adapter deliberately omits Measured, Delayed, and Degraded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlasmaObservationFrameExport {
    pub schema_version: String,
    pub shot_id: String,
    pub frame_id: String,
    pub clock_domain: String,
    pub frame_time_ns: u64,
    pub expected_channel_ids: Vec<String>,
    pub channels: Vec<DiagnosticChannelExport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticChannelExport {
    pub channel_id: String,
    pub diagnostic_system_id: String,
    pub disposition: ExportObservationDisposition,
}

/// Original sample-level replay state, preserved outside the shared
/// observation-frame schema so synthetic transformations cannot impersonate
/// physical sensor states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticReplayMetadata {
    pub sample_index: usize,
    pub sample_state: WaveformSampleState,
    pub source_sample_index: Option<usize>,
    pub active_fault_id: Option<String>,
    pub sample_time_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticObservationFrameExport {
    pub waveform_receipt_sha256: String,
    pub replay_metadata: SyntheticReplayMetadata,
    pub observation_frame: PlasmaObservationFrameExport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticObservationExportBody {
    pub schema_version: String,
    pub evidence_class: String,
    pub physical_measurements: bool,
    pub claim_ceiling: String,
    pub scenario_id: String,
    pub channel_id: String,
    pub units: String,
    pub model_id: String,
    pub model_revision: String,
    pub input_sha256: String,
    pub fault_schedule_sha256: String,
    pub waveform_receipt_sha256: String,
    pub frames: Vec<SyntheticObservationFrameExport>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticObservationExport {
    pub body: SyntheticObservationExportBody,
    pub export_sha256: String,
}

fn export_error(message: impl Into<String>) -> ReplayError {
    ReplayError::new(message)
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn disposition_for_sample(
    sample_state: WaveformSampleState,
    value: Option<f64>,
    units: &str,
    input_digest: &str,
    active_fault_id: Option<&str>,
) -> Result<ExportObservationDisposition, ReplayError> {
    match (sample_state, value) {
        (
            WaveformSampleState::SyntheticNominal
            | WaveformSampleState::Degraded
            | WaveformSampleState::Delayed,
            Some(value),
        ) => {
            if !value.is_finite() {
                return Err(export_error(
                    "synthetic observation export refuses non-finite values",
                ));
            }
            Ok(ExportObservationDisposition::Synthetic {
                value,
                units: units.to_owned(),
                model_id: SYNTHETIC_REPLAY_MODEL_ID.to_owned(),
                model_revision: SYNTHETIC_REPLAY_MODEL_REVISION.to_owned(),
                input_digest: input_digest.to_owned(),
            })
        }
        (WaveformSampleState::Delayed, None) => {
            let fault_id = active_fault_id.unwrap_or("unknown-delay-fault");
            Ok(ExportObservationDisposition::Unavailable {
                reason: format!("synthetic delay fault {fault_id} has no earlier source sample"),
            })
        }
        (WaveformSampleState::Unavailable, None) => {
            let fault_id = active_fault_id.unwrap_or("unknown-dropout-fault");
            Ok(ExportObservationDisposition::Unavailable {
                reason: format!("synthetic fault {fault_id} made the sample unavailable"),
            })
        }
        (WaveformSampleState::Invalid, None) => {
            let fault_id = active_fault_id.unwrap_or("unknown-invalid-fault");
            Ok(ExportObservationDisposition::Invalid {
                reason: format!("synthetic fault {fault_id} invalidated the sample"),
            })
        }
        _ => Err(export_error(
            "waveform sample state/value combination cannot be exported",
        )),
    }
}

/// Converts one verified waveform receipt into one Symthaea-compatible frame
/// per sample. The output can contain only synthetic, unavailable, or invalid
/// dispositions—never direct measured evidence.
pub fn export_synthetic_observation_frames(
    receipt: &WaveformReceipt,
) -> Result<SyntheticObservationExport, ReplayError> {
    verify_waveform_receipt(receipt)?;
    let body = &receipt.body;

    let mut frames = Vec::with_capacity(body.sample_count);
    for sample in &body.samples {
        let disposition = disposition_for_sample(
            sample.state,
            sample.value,
            &body.units,
            &body.input_sha256,
            sample.active_fault_id.as_deref(),
        )?;
        let frame_id = format!(
            "synthetic:{}:{}:{}",
            receipt.receipt_sha256, body.channel_id, sample.sample_index
        );

        frames.push(SyntheticObservationFrameExport {
            waveform_receipt_sha256: receipt.receipt_sha256.clone(),
            replay_metadata: SyntheticReplayMetadata {
                sample_index: sample.sample_index,
                sample_state: sample.state,
                source_sample_index: sample.source_sample_index,
                active_fault_id: sample.active_fault_id.clone(),
                sample_time_ns: sample.time_ns,
            },
            observation_frame: PlasmaObservationFrameExport {
                schema_version: PLASMA_OBSERVATION_FRAME_SCHEMA.to_owned(),
                shot_id: format!("synthetic-shot:{}", body.scenario_id),
                frame_id,
                clock_domain: "synthetic-waveform-relative-ns".to_owned(),
                frame_time_ns: sample.time_ns,
                expected_channel_ids: vec![body.channel_id.clone()],
                channels: vec![DiagnosticChannelExport {
                    channel_id: body.channel_id.clone(),
                    diagnostic_system_id: SYNTHETIC_REPLAY_DIAGNOSTIC_SYSTEM_ID.to_owned(),
                    disposition,
                }],
            },
        });
    }

    let export_body = SyntheticObservationExportBody {
        schema_version: OBSERVATION_EXPORT_SCHEMA.to_owned(),
        evidence_class: WAVEFORM_EVIDENCE_CLASS.to_owned(),
        physical_measurements: false,
        claim_ceiling: WAVEFORM_CLAIM_CEILING.to_owned(),
        scenario_id: body.scenario_id.clone(),
        channel_id: body.channel_id.clone(),
        units: body.units.clone(),
        model_id: SYNTHETIC_REPLAY_MODEL_ID.to_owned(),
        model_revision: SYNTHETIC_REPLAY_MODEL_REVISION.to_owned(),
        input_sha256: body.input_sha256.clone(),
        fault_schedule_sha256: body.fault_schedule_sha256.clone(),
        waveform_receipt_sha256: receipt.receipt_sha256.clone(),
        frames,
    };
    let export_sha256 = digest_serialized(&export_body)?;
    let export = SyntheticObservationExport {
        body: export_body,
        export_sha256,
    };
    verify_synthetic_observation_export(&export)?;
    Ok(export)
}

/// Verifies the export digest, claim ceiling, frame inventory and synthetic-only
/// state mapping. The waveform receipt is independently verified beforehand.
pub fn verify_synthetic_observation_export(
    export: &SyntheticObservationExport,
) -> Result<(), ReplayError> {
    let body = &export.body;
    if body.schema_version != OBSERVATION_EXPORT_SCHEMA {
        return Err(export_error(
            "unsupported synthetic observation export schema",
        ));
    }
    if body.evidence_class != WAVEFORM_EVIDENCE_CLASS
        || body.physical_measurements
        || body.claim_ceiling != WAVEFORM_CLAIM_CEILING
    {
        return Err(export_error(
            "synthetic observation export evidence class or claim ceiling was changed",
        ));
    }
    for (digest, field) in [
        (body.input_sha256.as_str(), "input_sha256"),
        (body.fault_schedule_sha256.as_str(), "fault_schedule_sha256"),
        (
            body.waveform_receipt_sha256.as_str(),
            "waveform_receipt_sha256",
        ),
    ] {
        if !valid_digest(digest) {
            return Err(export_error(format!(
                "synthetic observation export {field} is malformed"
            )));
        }
    }
    if body.scenario_id.trim().is_empty()
        || body.channel_id.trim().is_empty()
        || body.units.trim().is_empty()
        || body.model_id != SYNTHETIC_REPLAY_MODEL_ID
        || body.model_revision != SYNTHETIC_REPLAY_MODEL_REVISION
        || body.frames.is_empty()
    {
        return Err(export_error(
            "synthetic observation export metadata or frame inventory is incomplete",
        ));
    }

    let mut previous_time = None;
    for (index, frame_export) in body.frames.iter().enumerate() {
        let frame = &frame_export.observation_frame;
        let metadata = &frame_export.replay_metadata;
        if frame_export.waveform_receipt_sha256 != body.waveform_receipt_sha256
            || metadata.sample_index != index
            || metadata.sample_time_ns != frame.frame_time_ns
            || frame.schema_version != PLASMA_OBSERVATION_FRAME_SCHEMA
            || frame.shot_id != format!("synthetic-shot:{}", body.scenario_id)
            || frame.clock_domain != "synthetic-waveform-relative-ns"
            || frame.expected_channel_ids.len() != 1
            || frame.expected_channel_ids[0] != body.channel_id
            || frame.channels.len() != 1
            || frame.channels[0].channel_id != body.channel_id
            || frame.channels[0].diagnostic_system_id != SYNTHETIC_REPLAY_DIAGNOSTIC_SYSTEM_ID
            || frame.frame_id.trim().is_empty()
        {
            return Err(export_error(format!(
                "synthetic observation frame {index} does not match its envelope"
            )));
        }
        if let Some(previous) = previous_time
            && frame.frame_time_ns <= previous
        {
            return Err(export_error(
                "synthetic observation frame times must be strictly increasing",
            ));
        }
        previous_time = Some(frame.frame_time_ns);

        let disposition = &frame.channels[0].disposition;
        match (metadata.sample_state, disposition) {
            (
                WaveformSampleState::SyntheticNominal
                | WaveformSampleState::Degraded
                | WaveformSampleState::Delayed,
                ExportObservationDisposition::Synthetic {
                    value,
                    units,
                    model_id,
                    model_revision,
                    input_digest,
                },
            ) => {
                if !value.is_finite()
                    || units != &body.units
                    || model_id != &body.model_id
                    || model_revision != &body.model_revision
                    || input_digest != &body.input_sha256
                {
                    return Err(export_error(format!(
                        "synthetic frame {index} has inconsistent value or provenance metadata"
                    )));
                }
            }
            (
                WaveformSampleState::Delayed,
                ExportObservationDisposition::Unavailable { reason },
            ) if metadata.source_sample_index.is_none() && !reason.trim().is_empty() => {}
            (
                WaveformSampleState::Unavailable,
                ExportObservationDisposition::Unavailable { reason },
            ) if !reason.trim().is_empty() => {}
            (WaveformSampleState::Invalid, ExportObservationDisposition::Invalid { reason })
                if !reason.trim().is_empty() => {}
            _ => {
                return Err(export_error(format!(
                    "synthetic frame {index} disposition can promote or misclassify the replay state"
                )));
            }
        }

        if metadata.sample_state == WaveformSampleState::SyntheticNominal {
            if metadata.active_fault_id.is_some() || metadata.source_sample_index != Some(index) {
                return Err(export_error(
                    "synthetic nominal frame has inconsistent source/fault metadata",
                ));
            }
        } else if metadata
            .active_fault_id
            .as_deref()
            .is_none_or(str::is_empty)
        {
            return Err(export_error(
                "non-nominal synthetic frame must retain a non-empty fault identity",
            ));
        }

        if metadata.sample_state == WaveformSampleState::Delayed
            && metadata
                .source_sample_index
                .is_some_and(|source| source >= index)
        {
            return Err(export_error(
                "synthetic delayed frame must reference an earlier sample",
            ));
        }
    }

    if export.export_sha256 != digest_serialized(body)? {
        return Err(export_error("synthetic observation export digest mismatch"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::waveform::{
        WAVEFORM_SCHEMA_VERSION, WaveformFault, WaveformFaultMode, WaveformRequest,
        run_waveform_replay,
    };

    fn request_with_faults() -> WaveformRequest {
        WaveformRequest {
            schema_version: WAVEFORM_SCHEMA_VERSION.to_owned(),
            scenario_id: "synthetic-observation-export-shot".to_owned(),
            channel_id: "density".to_owned(),
            units: "synthetic-unit".to_owned(),
            time_origin_ns: 10_000,
            sample_interval_ns: 100,
            input_values: vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
            faults: vec![
                WaveformFault {
                    fault_id: "synthetic-bias".to_owned(),
                    start_sample: 1,
                    end_sample: 2,
                    mode: WaveformFaultMode::Bias { offset: 0.05 },
                },
                WaveformFault {
                    fault_id: "synthetic-delay".to_owned(),
                    start_sample: 2,
                    end_sample: 4,
                    mode: WaveformFaultMode::Delay { lag_samples: 1 },
                },
                WaveformFault {
                    fault_id: "synthetic-dropout".to_owned(),
                    start_sample: 4,
                    end_sample: 5,
                    mode: WaveformFaultMode::Dropout,
                },
                WaveformFault {
                    fault_id: "synthetic-integrity".to_owned(),
                    start_sample: 5,
                    end_sample: 6,
                    mode: WaveformFaultMode::IntegrityFailure,
                },
            ],
        }
    }

    #[test]
    fn waveform_receipt_exports_only_synthetic_unavailable_or_invalid_frames() {
        let receipt =
            run_waveform_replay(&request_with_faults()).expect("synthetic waveform replay");
        let export = export_synthetic_observation_frames(&receipt)
            .expect("export should produce observation frames");
        verify_synthetic_observation_export(&export).expect("export should verify");

        assert_eq!(export.body.frames.len(), 6);
        assert_eq!(
            export.body.frames[0].observation_frame.shot_id,
            "synthetic-shot:synthetic-observation-export-shot"
        );
        assert!(
            export.body.frames[0]
                .observation_frame
                .shot_id
                .starts_with("synthetic-shot:")
        );
        assert_eq!(export.body.evidence_class, WAVEFORM_EVIDENCE_CLASS);
        assert!(!export.body.physical_measurements);
        assert_eq!(export.body.waveform_receipt_sha256, receipt.receipt_sha256);

        assert!(matches!(
            export.body.frames[0].observation_frame.channels[0].disposition,
            ExportObservationDisposition::Synthetic { .. }
        ));
        assert!(matches!(
            export.body.frames[1].observation_frame.channels[0].disposition,
            ExportObservationDisposition::Synthetic { .. }
        ));
        assert!(matches!(
            export.body.frames[2].observation_frame.channels[0].disposition,
            ExportObservationDisposition::Synthetic { .. }
        ));
        assert_eq!(
            export.body.frames[2].replay_metadata.sample_state,
            WaveformSampleState::Delayed
        );
        assert!(matches!(
            export.body.frames[4].observation_frame.channels[0].disposition,
            ExportObservationDisposition::Unavailable { .. }
        ));
        assert!(matches!(
            export.body.frames[5].observation_frame.channels[0].disposition,
            ExportObservationDisposition::Invalid { .. }
        ));

        let json = serde_json::to_string(&export).expect("serialize export");
        let json_value: serde_json::Value =
            serde_json::from_str(&json).expect("inspect export JSON");
        let frames = json_value["body"]["frames"]
            .as_array()
            .expect("frame array");
        assert_eq!(frames.len(), 6);
        for frame in frames {
            assert_eq!(
                frame["observation_frame"]["schema_version"],
                PLASMA_OBSERVATION_FRAME_SCHEMA
            );
            assert_eq!(
                frame["observation_frame"]["expected_channel_ids"][0],
                "density"
            );
            let status = frame["observation_frame"]["channels"][0]["disposition"]["status"]
                .as_str()
                .expect("disposition status");
            assert!(matches!(status, "synthetic" | "unavailable" | "invalid"));
            assert_ne!(status, "measured");
            assert_ne!(status, "delayed");
            assert_ne!(status, "degraded");
        }
        let restored: SyntheticObservationExport =
            serde_json::from_str(&json).expect("roundtrip export");
        assert_eq!(restored, export);
        verify_synthetic_observation_export(&restored).expect("restored export verifies");
    }

    #[test]
    fn first_delayed_samples_without_a_source_are_unavailable_not_measured() {
        let mut request = request_with_faults();
        request.faults = vec![WaveformFault {
            fault_id: "synthetic-leading-delay".to_owned(),
            start_sample: 0,
            end_sample: 2,
            mode: WaveformFaultMode::Delay { lag_samples: 1 },
        }];
        let receipt = run_waveform_replay(&request).expect("delayed synthetic waveform");
        let export = export_synthetic_observation_frames(&receipt).expect("export");
        assert!(matches!(
            export.body.frames[0].observation_frame.channels[0].disposition,
            ExportObservationDisposition::Unavailable { .. }
        ));
        assert_eq!(
            export.body.frames[0].replay_metadata.sample_state,
            WaveformSampleState::Delayed
        );
        assert!(matches!(
            export.body.frames[1].observation_frame.channels[0].disposition,
            ExportObservationDisposition::Synthetic { .. }
        ));
    }

    #[test]
    fn export_verifier_rejects_measured_promotion_and_receipt_tampering() {
        let receipt =
            run_waveform_replay(&request_with_faults()).expect("synthetic waveform replay");
        let export = export_synthetic_observation_frames(&receipt).expect("synthetic export");
        let mut tampered = export.clone();
        tampered.body.frames[0].observation_frame.channels[0].disposition =
            ExportObservationDisposition::Unavailable {
                reason: "tampered".to_owned(),
            };
        tampered.export_sha256 = digest_serialized(&tampered.body).expect("reseal");
        assert!(verify_synthetic_observation_export(&tampered).is_err());

        let json = serde_json::to_string(&export).expect("serialize");
        let measured_json = json.replace("\"status\":\"synthetic\"", "\"status\":\"measured\"");
        let measured_result = serde_json::from_str::<SyntheticObservationExport>(&measured_json);
        assert!(measured_result.is_err());

        let mut promoted = export;
        promoted.body.physical_measurements = true;
        promoted.export_sha256 = digest_serialized(&promoted.body).expect("reseal");
        assert!(verify_synthetic_observation_export(&promoted).is_err());
    }
}
