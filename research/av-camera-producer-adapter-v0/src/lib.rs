// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Lossless bridge from the qualified software-camera receipt to AV replay.
//!
//! The adapter preserves the producer's policy receipt and clock milestones
//! while projecting the frame into the existing AV replay observation type.
//! It does not render pixels, resolve clocks, classify freshness, or read
//! evaluator-only world truth.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use action_conditioned_offscreen_camera::{
    PolicyObservationReceipt, RenderError, RenderedObservation, verify_policy_receipt,
};
use av_observation_replay_v0::{
    AgentObservationBundleV1, AgentObservationV1, ArtifactRefV1, AudioChunkObservationV1,
    BoundedTimeV1, CaptureIntervalV1, ClockDomainV1, ColorRangeV1, ColorSpaceV1,
    ContractError, PixelFormatV1, SCHEMA_VERSION_V1, TransportDispositionV1,
    TransportEventV1, VideoFrameObservationV1, validate_bundle_v1,
};
use serde::{Deserialize, Serialize};

/// Adapter schema version.
pub const ADAPTER_SCHEMA_VERSION_V1: u32 = 1;

/// Caller-supplied replay arrival facts.
///
/// These values are not inferred from producer microsecond timestamps. In
/// particular, availability_tick belongs to the replay clock and remains
/// separate from capture, render-completion and delivery timestamps.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayArrivalContextV1 {
    /// Unique transport event identity.
    pub event_id: String,
    /// Replay availability tick, supplied by the scenario/replay owner.
    pub availability_tick: u64,
    /// Deterministic arrival-order key, supplied by the replay owner.
    pub arrival_order: u64,
    /// Existing AV transport disposition; not a source-freshness label.
    pub disposition: TransportDispositionV1,
}

/// Inputs needed to adapt one camera frame.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CameraFrameAdaptationContextV1 {
    /// Stable AV stream/session identity.
    pub stream_id: String,
    /// Root-relative path at which the caller has persisted the exact RGB bytes.
    pub artifact_relative_path: String,
    /// Explicit source clock domain, generation and time-scale identity.
    pub clock_domain: ClockDomainV1,
    /// Replay/transport facts supplied independently of the camera clock.
    pub arrival: ReplayArrivalContextV1,
}

/// Capture/render/delivery milestones converted exactly to nanoseconds on the
/// explicitly supplied clock domain. Original microsecond values remain in
/// producer_receipt and are never overwritten.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CameraClockMilestonesV1 {
    /// Capture/acquisition timestamp in nanoseconds.
    pub capture_timestamp_ns: i64,
    /// Render-completion timestamp in nanoseconds.
    pub render_completed_timestamp_ns: i64,
    /// Delivery timestamp in nanoseconds.
    pub delivered_timestamp_ns: i64,
}

/// One adapted video frame plus its complete policy-visible producer receipt.
///
/// The evaluator-only camera world audit is intentionally not accepted here.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CameraFrameEnvelopeV1 {
    /// Adapter schema version.
    pub schema_version: u32,
    /// AV replay video observation.
    pub observation: VideoFrameObservationV1,
    /// AV replay transport event for this source observation.
    pub transport_event: TransportEventV1,
    /// Complete policy-visible receipt emitted by the camera producer.
    pub producer_receipt: PolicyObservationReceipt,
    /// Explicit source clock identity including reset generation.
    pub producer_clock_domain: ClockDomainV1,
    /// Separate capture/render/delivery milestones.
    pub clock_milestones: CameraClockMilestonesV1,
}

/// A batch that retains producer receipts beside the generic replay contract.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CameraObservationBatchV1 {
    /// Adapter schema version.
    pub schema_version: u32,
    /// Stream/session identity.
    pub stream_id: String,
    /// Adapted camera frames in source/catalogue order.
    pub frames: Vec<CameraFrameEnvelopeV1>,
}

/// Fail-closed adapter errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterError {
    /// Producer frame or unkeyed receipt commitment did not verify.
    ProducerReceiptRejected(RenderError),
    /// Artifact path is not a portable root-relative path.
    InvalidArtifactPath,
    /// Required identity is empty or inconsistent.
    InvalidIdentity,
    /// Clock domain is missing or does not match the producer clock ID.
    ClockDomainMismatch,
    /// Timestamp conversion from microseconds to nanoseconds overflowed.
    TimestampOverflow,
    /// A dropped transport event cannot carry a frame artifact.
    DroppedEventHasFrame,
    /// The generic AV replay validator rejected the projected bundle.
    ReplayContractRejected(ContractError),
    /// An adapted frame and its preserved producer receipt no longer agree.
    ProducerProjectionMismatch,
    /// Empty batch or invalid batch identity.
    InvalidBatch,
}

impl std::fmt::Display for AdapterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AdapterError {}

fn valid_root_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains('\0')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}

fn checked_us_to_ns(timestamp_us: u64) -> Result<i64, AdapterError> {
    let ns = timestamp_us
        .checked_mul(1_000)
        .ok_or(AdapterError::TimestampOverflow)?;
    i64::try_from(ns).map_err(|_| AdapterError::TimestampOverflow)
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Adapt a rendered RGB frame into the existing AV replay contract.
///
/// The caller must persist rendered.pixels_rgb at context.artifact_relative_path
/// before using the resulting ArtifactRef. The downstream #6201 artifact
/// loader remains responsible for root-confined byte loading and re-verification.
pub fn adapt_camera_frame_v1(
    rendered: &RenderedObservation,
    context: &CameraFrameAdaptationContextV1,
) -> Result<CameraFrameEnvelopeV1, AdapterError> {
    let receipt = &rendered.policy_receipt;
    verify_policy_receipt(receipt, &rendered.pixels_rgb)
        .map_err(AdapterError::ProducerReceiptRejected)?;

    if !valid_root_relative_path(&context.artifact_relative_path) {
        return Err(AdapterError::InvalidArtifactPath);
    }
    if !nonempty(&context.stream_id)
        || !nonempty(&context.arrival.event_id)
        || !nonempty(&context.clock_domain.domain_id)
        || !nonempty(&context.clock_domain.time_scale_id)
        || context.arrival.arrival_order == 0
    {
        return Err(AdapterError::InvalidIdentity);
    }
    if context.clock_domain.domain_id != receipt.clock_id {
        return Err(AdapterError::ClockDomainMismatch);
    }
    if context.arrival.disposition == TransportDispositionV1::Dropped {
        return Err(AdapterError::DroppedEventHasFrame);
    }
    if receipt.channels != 3
        || receipt.pixel_format
            != action_conditioned_offscreen_camera::PixelFormat::Rgb8Srgb
    {
        return Err(AdapterError::ProducerProjectionMismatch);
    }

    let capture_ns = checked_us_to_ns(receipt.capture_timestamp_us)?;
    let render_ns = checked_us_to_ns(receipt.render_completed_timestamp_us)?;
    let delivery_ns = checked_us_to_ns(receipt.delivered_timestamp_us)?;
    if capture_ns > render_ns || render_ns > delivery_ns {
        return Err(AdapterError::ProducerProjectionMismatch);
    }

    let stride_bytes = receipt
        .frame_width
        .checked_mul(3)
        .ok_or(AdapterError::ProducerProjectionMismatch)?;
    let observation = VideoFrameObservationV1 {
        observation_id: receipt.observation_id.clone(),
        sensor_id: receipt.camera_id.clone(),
        source_sequence: receipt.sequence,
        artifact: ArtifactRefV1 {
            relative_path: context.artifact_relative_path.clone(),
            sha256: receipt.frame_sha256.clone(),
            byte_len: u64::try_from(receipt.frame_byte_len)
                .map_err(|_| AdapterError::ProducerProjectionMismatch)?,
        },
        width: receipt.frame_width,
        height: receipt.frame_height,
        stride_bytes,
        pixel_format: PixelFormatV1::Rgb8,
        color_space: ColorSpaceV1::Srgb,
        color_range: ColorRangeV1::Full,
        camera_profile_sha256: receipt.camera_calibration_sha256.clone(),
        // This synthetic renderer's geometry digest is not a physical
        // calibration certificate. Keep the calibration authority absent.
        calibration_profile_sha256: None,
        clock_domain: context.clock_domain.clone(),
        // #4616 currently supplies one sample timestamp, not an exposure
        // interval. Preserve it as a point interval; do not invent exposure.
        capture_interval: CaptureIntervalV1 {
            start: BoundedTimeV1 {
                lower_ns: capture_ns,
                upper_ns: capture_ns,
            },
            end: BoundedTimeV1 {
                lower_ns: capture_ns,
                upper_ns: capture_ns,
            },
        },
    };

    let transport_event = TransportEventV1 {
        event_id: context.arrival.event_id.clone(),
        sensor_id: receipt.camera_id.clone(),
        source_sequence: receipt.sequence,
        clock_domain: context.clock_domain.clone(),
        availability_tick: context.arrival.availability_tick,
        arrival_order: context.arrival.arrival_order,
        disposition: context.arrival.disposition,
        observation_id: Some(receipt.observation_id.clone()),
    };

    let envelope = CameraFrameEnvelopeV1 {
        schema_version: ADAPTER_SCHEMA_VERSION_V1,
        observation,
        transport_event,
        producer_receipt: receipt.clone(),
        producer_clock_domain: context.clock_domain.clone(),
        clock_milestones: CameraClockMilestonesV1 {
            capture_timestamp_ns: capture_ns,
            render_completed_timestamp_ns: render_ns,
            delivered_timestamp_ns: delivery_ns,
        },
    };
    validate_camera_frame_envelope_v1(&envelope)?;
    Ok(envelope)
}

/// Validate an envelope against both its preserved producer receipt and the
/// existing AV replay contract.
pub fn validate_camera_frame_envelope_v1(
    envelope: &CameraFrameEnvelopeV1,
) -> Result<(), AdapterError> {
    if envelope.schema_version != ADAPTER_SCHEMA_VERSION_V1 {
        return Err(AdapterError::ProducerProjectionMismatch);
    }
    let receipt = &envelope.producer_receipt;
    if envelope.observation.observation_id != receipt.observation_id
        || envelope.observation.sensor_id != receipt.camera_id
        || envelope.observation.source_sequence != receipt.sequence
        || envelope.observation.artifact.sha256 != receipt.frame_sha256
        || envelope.observation.artifact.byte_len
            != u64::try_from(receipt.frame_byte_len)
                .map_err(|_| AdapterError::ProducerProjectionMismatch)?
        || envelope.observation.width != receipt.frame_width
        || envelope.observation.height != receipt.frame_height
        || envelope.producer_clock_domain.domain_id != receipt.clock_id
        || envelope.transport_event.observation_id.as_deref()
            != Some(receipt.observation_id.as_str())
        || envelope.transport_event.source_sequence != receipt.sequence
        || envelope.transport_event.sensor_id != receipt.camera_id
        || envelope.transport_event.clock_domain != envelope.producer_clock_domain
    {
        return Err(AdapterError::ProducerProjectionMismatch);
    }

    let expected_milestones = CameraClockMilestonesV1 {
        capture_timestamp_ns: checked_us_to_ns(receipt.capture_timestamp_us)?,
        render_completed_timestamp_ns: checked_us_to_ns(receipt.render_completed_timestamp_us)?,
        delivered_timestamp_ns: checked_us_to_ns(receipt.delivered_timestamp_us)?,
    };
    if envelope.clock_milestones != expected_milestones {
        return Err(AdapterError::ProducerProjectionMismatch);
    }
    Ok(())
}

impl CameraObservationBatchV1 {
    /// Project a receipt-preserving batch into the existing generic replay bundle.
    ///
    /// Producer receipts remain in this batch as sidecars; the returned replay
    /// bundle intentionally carries only the established #6201 observation and
    /// transport contract.
    pub fn to_replay_bundle_v1(&self) -> Result<AgentObservationBundleV1, AdapterError> {
        if self.schema_version != ADAPTER_SCHEMA_VERSION_V1
            || !nonempty(&self.stream_id)
            || self.frames.is_empty()
        {
            return Err(AdapterError::InvalidBatch);
        }
        for frame in &self.frames {
            validate_camera_frame_envelope_v1(frame)?;
        }
        let bundle = AgentObservationBundleV1 {
            schema_version: SCHEMA_VERSION_V1,
            stream_id: self.stream_id.clone(),
            observations: self
                .frames
                .iter()
                .map(|frame| AgentObservationV1::VideoFrame(frame.observation.clone()))
                .collect(),
            transport_events: self
                .frames
                .iter()
                .map(|frame| frame.transport_event.clone())
                .collect(),
        };
        validate_bundle_v1(&bundle).map_err(AdapterError::ReplayContractRejected)?;
        Ok(bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_conditioned_offscreen_camera::{
        AppliedActionRef, CameraCalibration, RenderClocks, SoftwareCameraRenderer,
        WorldRenderSnapshot, Q16_ONE,
    };

    fn snapshot(timestamp_us: u64) -> WorldRenderSnapshot {
        WorldRenderSnapshot {
            episode_id: "adapter-episode".to_owned(),
            scenario_id: "adapter-scenario".to_owned(),
            agent_id: "adapter-agent".to_owned(),
            tick: 1,
            world_timestamp_us: timestamp_us,
            clock_id: "sim-clock".to_owned(),
            simulator_revision: "software-scene-v1".to_owned(),
            applied_action: Some(AppliedActionRef {
                receipt_id: "action-receipt-1".to_owned(),
                receipt_sha256: "a".repeat(64),
                command_id: "action-command-1".to_owned(),
                applied_tick: 1,
            }),
            camera: CameraCalibration {
                camera_id: "front-rgb".to_owned(),
                position_world_mm: [0, 0, 1_500],
                world_to_camera_q16: [
                    [Q16_ONE as i32, 0, 0],
                    [0, 0, -(Q16_ONE as i32)],
                    [0, Q16_ONE as i32, 0],
                ],
                focal_x_millipx: 10_000,
                focal_y_millipx: 10_000,
                principal_x_millipx: 8_000,
                principal_y_millipx: 8_000,
                width: 16,
                height: 16,
                near_plane_mm: 100,
                far_plane_mm: 200_000,
            },
            road_markings: Vec::new(),
            actors: Vec::new(),
        }
    }

    fn rendered_at(timestamp_us: u64) -> RenderedObservation {
        let snapshot = snapshot(timestamp_us);
        SoftwareCameraRenderer::render(
            &snapshot,
            1,
            RenderClocks {
                capture_timestamp_us: timestamp_us,
                render_completed_timestamp_us: timestamp_us + 20,
                delivered_timestamp_us: timestamp_us + 50,
            },
        )
        .unwrap()
    }

    fn context() -> CameraFrameAdaptationContextV1 {
        CameraFrameAdaptationContextV1 {
            stream_id: "adapter-stream".to_owned(),
            artifact_relative_path: "camera/front/000001.rgb".to_owned(),
            clock_domain: ClockDomainV1 {
                domain_id: "sim-clock".to_owned(),
                generation: 0,
                time_scale_id: "simulation-monotonic-v1".to_owned(),
            },
            arrival: ReplayArrivalContextV1 {
                event_id: "transport-event-1".to_owned(),
                availability_tick: 7,
                arrival_order: 1,
                disposition: TransportDispositionV1::Delivered,
            },
        }
    }

    #[test]
    fn adapts_real_camera_output_and_preserves_all_three_clock_milestones() {
        let rendered = rendered_at(10_000);
        let envelope = adapt_camera_frame_v1(&rendered, &context()).unwrap();
        assert_eq!(envelope.observation.pixel_format, PixelFormatV1::Rgb8);
        assert_eq!(envelope.observation.color_space, ColorSpaceV1::Srgb);
        assert_eq!(envelope.observation.stride_bytes, 48);
        assert_eq!(envelope.observation.capture_interval.start.lower_ns, 10_000_000);
        assert_eq!(envelope.clock_milestones.capture_timestamp_ns, 10_000_000);
        assert_eq!(envelope.clock_milestones.render_completed_timestamp_ns, 10_020_000);
        assert_eq!(envelope.clock_milestones.delivered_timestamp_ns, 10_050_000);
        assert_eq!(envelope.producer_receipt, rendered.policy_receipt);
        assert_eq!(envelope.transport_event.availability_tick, 7);
        assert_ne!(envelope.transport_event.availability_tick, 10_050_000);
        assert_eq!(envelope.observation.calibration_profile_sha256, None);
    }

    #[test]
    fn pixel_mutation_is_rejected_before_an_artifact_reference_is_emitted() {
        let mut rendered = rendered_at(10_000);
        rendered.pixels_rgb[0] ^= 1;
        assert!(matches!(
            adapt_camera_frame_v1(&rendered, &context()),
            Err(AdapterError::ProducerReceiptRejected(
                RenderError::ParentFrameMismatch
            ))
        ));
    }

    #[test]
    fn producer_clock_identity_must_be_explicit_and_match() {
        let mut ctx = context();
        ctx.clock_domain.generation = 1;
        assert_eq!(
            adapt_camera_frame_v1(&rendered_at(10_000), &ctx),
            Err(AdapterError::ClockDomainMismatch)
        );
    }

    #[test]
    fn artifact_paths_must_be_portable_and_root_relative() {
        for invalid in ["/tmp/frame.rgb", "../frame.rgb", "camera/../frame.rgb", "camera\\frame.rgb"] {
            let mut ctx = context();
            ctx.artifact_relative_path = invalid.to_owned();
            assert_eq!(
                adapt_camera_frame_v1(&rendered_at(10_000), &ctx),
                Err(AdapterError::InvalidArtifactPath)
            );
        }
    }

    #[test]
    fn transport_availability_is_not_inferred_from_camera_microseconds() {
        let mut ctx = context();
        ctx.arrival.availability_tick = 123;
        let envelope = adapt_camera_frame_v1(&rendered_at(10_000), &ctx).unwrap();
        assert_eq!(envelope.transport_event.availability_tick, 123);
        assert_eq!(envelope.producer_receipt.delivered_timestamp_us, 10_050);
    }

    #[test]
    fn dropped_event_cannot_be_attached_to_a_frame_payload() {
        let mut ctx = context();
        ctx.arrival.disposition = TransportDispositionV1::Dropped;
        assert_eq!(
            adapt_camera_frame_v1(&rendered_at(10_000), &ctx),
            Err(AdapterError::DroppedEventHasFrame)
        );
    }

    #[test]
    fn microsecond_to_nanosecond_overflow_rejects() {
        let timestamp_us = (i64::MAX as u64 / 1_000) + 1;
        let rendered = rendered_at(timestamp_us);
        assert_eq!(
            adapt_camera_frame_v1(&rendered, &context()),
            Err(AdapterError::TimestampOverflow)
        );
    }

    #[test]
    fn generic_replay_bundle_passes_the_existing_contract() {
        let rendered = rendered_at(10_000);
        let envelope = adapt_camera_frame_v1(&rendered, &context()).unwrap();
        let batch = CameraObservationBatchV1 {
            schema_version: ADAPTER_SCHEMA_VERSION_V1,
            stream_id: "adapter-stream".to_owned(),
            frames: vec![envelope],
        };
        let bundle = batch.to_replay_bundle_v1().unwrap();
        assert_eq!(bundle.observations.len(), 1);
        assert_eq!(bundle.transport_events.len(), 1);
        assert_eq!(bundle.stream_id, "adapter-stream");
    }

    #[test]
    fn producer_receipt_serializes_without_evaluator_world_audit() {
        let rendered = rendered_at(10_000);
        let envelope = adapt_camera_frame_v1(&rendered, &context()).unwrap();
        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains("render_completed_timestamp_us"));
        assert!(json.contains("applied_action_receipt_id"));
        assert!(!json.contains("scenario_id"));
        assert!(!json.contains("actor_ids"));
        assert!(!json.contains("world_snapshot_sha256"));
    }

    #[test]
    fn adapted_frame_rejects_mutated_sidecar_or_transport_projection() {
        let rendered = rendered_at(10_000);
        let mut envelope = adapt_camera_frame_v1(&rendered, &context()).unwrap();
        envelope.clock_milestones.delivered_timestamp_ns += 1;
        assert_eq!(
            validate_camera_frame_envelope_v1(&envelope),
            Err(AdapterError::ProducerProjectionMismatch)
        );
    }
}
