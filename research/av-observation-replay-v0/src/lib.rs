// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Strict agent-visible audiovisual artifacts and deterministic causal replay.
//!
//! This crate defines metadata and delivery semantics only. It does not render
//! pixels or waveforms, transform clocks, infer world truth, or isolate
//! evaluator state at an operating-system boundary.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current schema version admitted by this crate.
pub const SCHEMA_VERSION_V1: u32 = 1;

/// Immutable reference to one raw or derived sensor artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRefV1 {
    /// Safe path relative to the caller-supplied artifact root.
    pub relative_path: String,
    /// Lowercase SHA-256 hex digest of the exact artifact bytes.
    pub sha256: String,
    /// Exact byte length of the artifact.
    pub byte_len: u64,
}

/// Bounded timestamp interval in integer nanoseconds on one declared time scale.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundedTimeV1 {
    /// Earliest possible timestamp.
    pub lower_ns: i64,
    /// Latest possible timestamp.
    pub upper_ns: i64,
}

/// Capture interval with bounded start and end timestamps.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureIntervalV1 {
    /// Bounded capture start.
    pub start: BoundedTimeV1,
    /// Bounded capture end.
    pub end: BoundedTimeV1,
}

/// Identity of a source clock, including reset generation and time scale.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClockDomainV1 {
    /// Stable source clock identity.
    pub domain_id: String,
    /// Incremented whenever the source clock resets or is reinitialized.
    pub generation: u64,
    /// Explicit time-scale identity; not inferred from numeric timestamps.
    pub time_scale_id: String,
}

/// Relation between two declared source clocks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockRelationV1 {
    /// Both observations use exactly the same clock domain and generation.
    SameClockGeneration,
    /// No admitted transform exists in this crate for the two clock identities.
    Incomparable,
}

/// Classify clock comparability without inferring offsets, drift or simultaneity.
pub fn clock_relation_v1(left: &ClockDomainV1, right: &ClockDomainV1) -> ClockRelationV1 {
    if left == right {
        ClockRelationV1::SameClockGeneration
    } else {
        ClockRelationV1::Incomparable
    }
}

/// Pixel layout for a raw video-frame artifact.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelFormatV1 {
    /// One byte per pixel.
    Gray8,
    /// Three bytes per pixel in red, green, blue order.
    Rgb8,
    /// Four bytes per pixel in red, green, blue, alpha order.
    Rgba8,
}

impl PixelFormatV1 {
    fn bytes_per_pixel(self) -> u64 {
        match self {
            Self::Gray8 => 1,
            Self::Rgb8 => 3,
            Self::Rgba8 => 4,
        }
    }
}

/// Color-space declaration for a video frame.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorSpaceV1 {
    /// Standard RGB transfer/color space.
    Srgb,
    /// ITU-R BT.709.
    Bt709,
    /// ITU-R BT.2020.
    Bt2020,
}

/// Numeric range declaration for a video frame.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColorRangeV1 {
    /// Full code-value range.
    Full,
    /// Limited/video code-value range.
    Limited,
}

/// One captured video frame as an immutable artifact plus source metadata.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VideoFrameObservationV1 {
    /// Unique observation identity within the bundle.
    pub observation_id: String,
    /// Sensor identity declared by the capture profile.
    pub sensor_id: String,
    /// Source frame sequence, not a timestamp.
    pub source_sequence: u64,
    /// Raw pixel artifact.
    pub artifact: ArtifactRefV1,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Row stride in bytes, including any row padding.
    pub stride_bytes: u32,
    /// Raw pixel format.
    pub pixel_format: PixelFormatV1,
    /// Color-space declaration.
    pub color_space: ColorSpaceV1,
    /// Color-range declaration.
    pub color_range: ColorRangeV1,
    /// SHA-256 identity of the camera/sensor profile.
    pub camera_profile_sha256: String,
    /// Optional calibration identity; absent means calibration is not established.
    pub calibration_profile_sha256: Option<String>,
    /// Source clock identity.
    pub clock_domain: ClockDomainV1,
    /// Bounded source capture interval.
    pub capture_interval: CaptureIntervalV1,
}

/// Sample encoding for a raw PCM chunk.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PcmFormatV1 {
    /// Signed 16-bit little-endian PCM.
    S16Le,
    /// IEEE-754 32-bit little-endian PCM.
    F32Le,
}

impl PcmFormatV1 {
    fn bytes_per_sample(self) -> u64 {
        match self {
            Self::S16Le => 2,
            Self::F32Le => 4,
        }
    }
}

/// Declared channel role in an audio chunk.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioChannelV1 {
    /// Single-channel/mono signal.
    Mono,
    /// Front-left channel.
    FrontLeft,
    /// Front-right channel.
    FrontRight,
    /// Front-center channel.
    FrontCenter,
    /// Low-frequency effects channel.
    Lfe,
    /// Rear-left channel.
    RearLeft,
    /// Rear-right channel.
    RearRight,
    /// Side-left channel.
    SideLeft,
    /// Side-right channel.
    SideRight,
    /// Ambisonic W component.
    AmbisonicW,
    /// Ambisonic X component.
    AmbisonicX,
    /// Ambisonic Y component.
    AmbisonicY,
    /// Ambisonic Z component.
    AmbisonicZ,
}

/// One PCM chunk with absolute half-open source-sample coordinates.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AudioChunkObservationV1 {
    /// Unique observation identity within the bundle.
    pub observation_id: String,
    /// Sensor identity declared by the capture profile.
    pub sensor_id: String,
    /// Source chunk sequence, not a timestamp.
    pub source_sequence: u64,
    /// Raw PCM artifact.
    pub artifact: ArtifactRefV1,
    /// PCM sample encoding.
    pub sample_format: PcmFormatV1,
    /// Source sampling rate in samples per second.
    pub sample_rate_hz: u32,
    /// Ordered channel layout; duplicates are invalid.
    pub channels: Vec<AudioChannelV1>,
    /// Inclusive first sample index in the source clock generation.
    pub source_start_sample: u64,
    /// Exclusive sample index after the final sample.
    pub source_end_sample: u64,
    /// SHA-256 identity of the microphone/device profile.
    pub device_profile_sha256: String,
    /// Optional calibration identity; absent means calibration is not established.
    pub calibration_profile_sha256: Option<String>,
    /// Source clock identity.
    pub clock_domain: ClockDomainV1,
    /// Bounded source capture interval.
    pub capture_interval: CaptureIntervalV1,
}

/// One modality-specific observation delivered to the agent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "observation", rename_all = "snake_case")]
pub enum AgentObservationV1 {
    /// A raw visual frame.
    VideoFrame(VideoFrameObservationV1),
    /// A raw audio chunk.
    AudioChunk(AudioChunkObservationV1),
}

impl AgentObservationV1 {
    /// Unique identity within the observation bundle.
    pub fn observation_id(&self) -> &str {
        match self {
            Self::VideoFrame(value) => &value.observation_id,
            Self::AudioChunk(value) => &value.observation_id,
        }
    }

    /// Source sensor identity.
    pub fn sensor_id(&self) -> &str {
        match self {
            Self::VideoFrame(value) => &value.sensor_id,
            Self::AudioChunk(value) => &value.sensor_id,
        }
    }

    /// Source sequence number.
    pub fn source_sequence(&self) -> u64 {
        match self {
            Self::VideoFrame(value) => value.source_sequence,
            Self::AudioChunk(value) => value.source_sequence,
        }
    }

    /// Source clock identity.
    pub fn clock_domain(&self) -> &ClockDomainV1 {
        match self {
            Self::VideoFrame(value) => &value.clock_domain,
            Self::AudioChunk(value) => &value.clock_domain,
        }
    }

    /// Immutable artifact reference for this observation.
    pub fn artifact_ref(&self) -> &ArtifactRefV1 {
        match self {
            Self::VideoFrame(value) => &value.artifact,
            Self::AudioChunk(value) => &value.artifact,
        }
    }
}

/// How the transport layer handled one source observation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportDispositionV1 {
    /// First delivery in strictly increasing source-sequence order.
    Delivered,
    /// First delivery arrived after a higher source sequence from the same sensor.
    Reordered,
    /// A previously delivered observation was delivered again.
    Duplicate,
    /// A previously delivered observation was replayed as stale.
    StaleReplay,
    /// A source sequence was expected but no observation payload was delivered.
    Dropped,
}

/// One arrival/drop event in the agent-visible transport schedule.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransportEventV1 {
    /// Unique transport-event identity.
    pub event_id: String,
    /// Source sensor identity.
    pub sensor_id: String,
    /// Source frame/chunk sequence number.
    pub source_sequence: u64,
    /// Earliest replay tick at which this transport event becomes visible.
    pub availability_tick: u64,
    /// Globally unique deterministic arrival-order key.
    pub arrival_order: u64,
    /// Explicit transport disposition.
    pub disposition: TransportDispositionV1,
    /// Observation payload identity, absent only for a Dropped event.
    pub observation_id: Option<String>,
}

/// Complete agent-visible observation and transport bundle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentObservationBundleV1 {
    /// Schema version; must equal SCHEMA_VERSION_V1.
    pub schema_version: u32,
    /// Stable synthetic stream/session identity.
    pub stream_id: String,
    /// All unique sensor artifacts available to this agent session.
    pub observations: Vec<AgentObservationV1>,
    /// Delivery schedule including explicit loss, reordering and replay events.
    pub transport_events: Vec<TransportEventV1>,
}

/// Validation failures for malformed or semantically inconsistent bundles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractError {
    /// Unsupported schema version.
    UnsupportedSchemaVersion,
    /// Required identity string is empty.
    EmptyIdentity,
    /// Artifact path is absolute, traversing, or otherwise unsafe.
    InvalidArtifactPath,
    /// SHA-256 field is not 64 lowercase hexadecimal characters.
    InvalidDigest,
    /// Artifact has zero declared bytes.
    EmptyArtifact,
    /// Video dimensions, stride or declared byte length are inconsistent.
    InvalidVideoShape,
    /// Audio rate, channels, sample range or declared byte length is inconsistent.
    InvalidAudioShape,
    /// Capture interval is invalid or has reversed bounds.
    InvalidCaptureInterval,
    /// Clock-domain identity is incomplete.
    InvalidClockDomain,
    /// Calibration digest syntax is invalid.
    InvalidCalibrationDigest,
    /// Observation IDs must be unique.
    DuplicateObservationId,
    /// Transport event IDs must be unique.
    DuplicateTransportEventId,
    /// Arrival-order keys must be unique.
    DuplicateArrivalOrder,
    /// Availability ticks must be nondecreasing in arrival order.
    AvailabilityOrderRegression,
    /// Transport event references a missing observation.
    MissingObservationReference,
    /// Transport event has a payload when it should not.
    UnexpectedObservationReference,
    /// Transport sensor/sequence does not match its referenced observation.
    ObservationReferenceMismatch,
    /// Disposition does not match first delivery/replay semantics.
    InvalidTransportSemantics,
    /// At least one observation has no first-delivery transport event.
    MissingFirstDelivery,
    /// Replay time moved backwards.
    ReplayTimeRegression,
    /// Artifact payload bytes do not match committed length or digest.
    ArtifactIntegrityMismatch,
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ContractError {}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_relative_path(value: &str) -> bool {
    if !nonempty(value)
        || value.starts_with('/')
        || value.contains('\\')
        || value.contains(':')
    {
        return false;
    }
    value
        .split('/')
        .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn validate_artifact(artifact: &ArtifactRefV1) -> Result<(), ContractError> {
    if !valid_relative_path(&artifact.relative_path) {
        return Err(ContractError::InvalidArtifactPath);
    }
    if !is_sha256(&artifact.sha256) {
        return Err(ContractError::InvalidDigest);
    }
    if artifact.byte_len == 0 {
        return Err(ContractError::EmptyArtifact);
    }
    Ok(())
}

fn validate_clock(clock: &ClockDomainV1) -> Result<(), ContractError> {
    if !nonempty(&clock.domain_id) || !nonempty(&clock.time_scale_id) {
        return Err(ContractError::InvalidClockDomain);
    }
    Ok(())
}

fn validate_capture_interval(interval: CaptureIntervalV1) -> Result<(), ContractError> {
    let start = interval.start;
    let end = interval.end;
    if start.lower_ns > start.upper_ns
        || end.lower_ns > end.upper_ns
        || start.lower_ns > end.lower_ns
        || start.upper_ns > end.upper_ns
    {
        return Err(ContractError::InvalidCaptureInterval);
    }
    Ok(())
}

fn validate_optional_digest(value: Option<&str>) -> Result<(), ContractError> {
    if value.is_some_and(|digest| !is_sha256(digest)) {
        return Err(ContractError::InvalidCalibrationDigest);
    }
    Ok(())
}

fn validate_observation(observation: &AgentObservationV1) -> Result<(), ContractError> {
    match observation {
        AgentObservationV1::VideoFrame(frame) => {
            if !nonempty(&frame.observation_id)
                || !nonempty(&frame.sensor_id)
                || !is_sha256(&frame.camera_profile_sha256)
            {
                return Err(ContractError::EmptyIdentity);
            }
            validate_artifact(&frame.artifact)?;
            validate_clock(&frame.clock_domain)?;
            validate_capture_interval(frame.capture_interval)?;
            validate_optional_digest(frame.calibration_profile_sha256.as_deref())?;

            let minimum_stride = u64::from(frame.width)
                .checked_mul(frame.pixel_format.bytes_per_pixel())
                .ok_or(ContractError::InvalidVideoShape)?;
            let expected_len = u64::from(frame.stride_bytes)
                .checked_mul(u64::from(frame.height))
                .ok_or(ContractError::InvalidVideoShape)?;
            if frame.width == 0
                || frame.height == 0
                || u64::from(frame.stride_bytes) < minimum_stride
                || expected_len != frame.artifact.byte_len
            {
                return Err(ContractError::InvalidVideoShape);
            }
        }
        AgentObservationV1::AudioChunk(chunk) => {
            if !nonempty(&chunk.observation_id)
                || !nonempty(&chunk.sensor_id)
                || !is_sha256(&chunk.device_profile_sha256)
            {
                return Err(ContractError::EmptyIdentity);
            }
            validate_artifact(&chunk.artifact)?;
            validate_clock(&chunk.clock_domain)?;
            validate_capture_interval(chunk.capture_interval)?;
            validate_optional_digest(chunk.calibration_profile_sha256.as_deref())?;

            let sample_count = chunk
                .source_end_sample
                .checked_sub(chunk.source_start_sample)
                .ok_or(ContractError::InvalidAudioShape)?;
            let channel_count =
                u64::try_from(chunk.channels.len()).map_err(|_| ContractError::InvalidAudioShape)?;
            let expected_len = sample_count
                .checked_mul(channel_count)
                .and_then(|value| value.checked_mul(chunk.sample_format.bytes_per_sample()))
                .ok_or(ContractError::InvalidAudioShape)?;
            let unique_channels: BTreeSet<_> = chunk.channels.iter().copied().collect();
            if chunk.sample_rate_hz == 0
                || chunk.source_end_sample <= chunk.source_start_sample
                || chunk.channels.is_empty()
                || unique_channels.len() != chunk.channels.len()
                || expected_len != chunk.artifact.byte_len
            {
                return Err(ContractError::InvalidAudioShape);
            }
        }
    }
    Ok(())
}

/// Validate schema, artifact declarations and transport semantics.
///
/// Actual artifact bytes are checked separately with verify_artifact_bytes_v1.
pub fn validate_bundle_v1(bundle: &AgentObservationBundleV1) -> Result<(), ContractError> {
    if bundle.schema_version != SCHEMA_VERSION_V1 {
        return Err(ContractError::UnsupportedSchemaVersion);
    }
    if !nonempty(&bundle.stream_id) || bundle.observations.is_empty() {
        return Err(ContractError::EmptyIdentity);
    }

    let mut observation_by_id: BTreeMap<String, (&str, u64)> = BTreeMap::new();
    for observation in &bundle.observations {
        validate_observation(observation)?;
        let id = observation.observation_id().to_owned();
        if observation_by_id
            .insert(id, (observation.sensor_id(), observation.source_sequence()))
            .is_some()
        {
            return Err(ContractError::DuplicateObservationId);
        }
    }

    let mut event_ids = BTreeSet::new();
    let mut arrival_orders = BTreeSet::new();
    for event in &bundle.transport_events {
        if !nonempty(&event.event_id) || !nonempty(&event.sensor_id) {
            return Err(ContractError::EmptyIdentity);
        }
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(ContractError::DuplicateTransportEventId);
        }
        if !arrival_orders.insert(event.arrival_order) {
            return Err(ContractError::DuplicateArrivalOrder);
        }
    }

    let mut ordered_events: Vec<&TransportEventV1> = bundle.transport_events.iter().collect();
    ordered_events.sort_by_key(|event| event.arrival_order);

    let mut prior_availability = None;
    let mut high_water: BTreeMap<String, u64> = BTreeMap::new();
    let mut first_deliveries = BTreeSet::new();
    let mut delivered_ids: BTreeMap<String, (String, u64)> = BTreeMap::new();

    for event in ordered_events {
        if prior_availability.is_some_and(|tick| event.availability_tick < tick) {
            return Err(ContractError::AvailabilityOrderRegression);
        }
        prior_availability = Some(event.availability_tick);

        match event.disposition {
            TransportDispositionV1::Dropped => {
                if event.observation_id.is_some() {
                    return Err(ContractError::UnexpectedObservationReference);
                }
                let high = high_water.entry(event.sensor_id.clone()).or_default();
                *high = (*high).max(event.source_sequence);
            }
            TransportDispositionV1::Delivered | TransportDispositionV1::Reordered => {
                let id = event
                    .observation_id
                    .as_ref()
                    .ok_or(ContractError::MissingObservationReference)?;
                let (sensor_id, source_sequence) = observation_by_id
                    .get(id)
                    .ok_or(ContractError::MissingObservationReference)?;
                if *sensor_id != event.sensor_id.as_str() || *source_sequence != event.source_sequence {
                    return Err(ContractError::ObservationReferenceMismatch);
                }
                if delivered_ids.contains_key(id) {
                    return Err(ContractError::InvalidTransportSemantics);
                }
                let high = high_water.get(&event.sensor_id).copied();
                let valid_first_delivery = match event.disposition {
                    TransportDispositionV1::Delivered => {
                        !high.is_some_and(|previous| event.source_sequence <= previous)
                    }
                    TransportDispositionV1::Reordered => {
                        high.is_some_and(|previous| event.source_sequence < previous)
                    }
                    _ => false,
                };
                if !valid_first_delivery {
                    return Err(ContractError::InvalidTransportSemantics);
                }
                high_water
                    .entry(event.sensor_id.clone())
                    .and_modify(|previous| *previous = (*previous).max(event.source_sequence))
                    .or_insert(event.source_sequence);
                first_deliveries.insert(id.clone());
                delivered_ids.insert(id.clone(), (event.sensor_id.clone(), event.source_sequence));
            }
            TransportDispositionV1::Duplicate | TransportDispositionV1::StaleReplay => {
                let id = event
                    .observation_id
                    .as_ref()
                    .ok_or(ContractError::MissingObservationReference)?;
                let (sensor_id, source_sequence) = delivered_ids
                    .get(id)
                    .ok_or(ContractError::InvalidTransportSemantics)?;
                if sensor_id != &event.sensor_id || *source_sequence != event.source_sequence {
                    return Err(ContractError::ObservationReferenceMismatch);
                }
            }
        }
    }

    if first_deliveries.len() != observation_by_id.len() {
        return Err(ContractError::MissingFirstDelivery);
    }
    Ok(())
}

/// Compute a lowercase SHA-256 digest for exact artifact bytes.
pub fn sha256_hex_v1(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

/// Verify payload length and SHA-256 against an artifact reference.
pub fn verify_artifact_bytes_v1(
    reference: &ArtifactRefV1,
    bytes: &[u8],
) -> Result<(), ContractError> {
    validate_artifact(reference)?;
    if u64::try_from(bytes.len()).ok() != Some(reference.byte_len)
        || sha256_hex_v1(bytes) != reference.sha256
    {
        return Err(ContractError::ArtifactIntegrityMismatch);
    }
    Ok(())
}

/// One causally available transport event and its optional observation payload.
#[derive(Clone, Debug)]
pub struct ReplayDeliveryV1<'a> {
    /// Transport event becoming available now.
    pub transport: &'a TransportEventV1,
    /// Referenced sensor observation, absent for a dropped event.
    pub observation: Option<&'a AgentObservationV1>,
}

/// Deterministic causal replay cursor over one validated observation bundle.
pub struct ReplayCursorV1<'a> {
    bundle: &'a AgentObservationBundleV1,
    ordered_event_indices: Vec<usize>,
    next_event: usize,
    last_tick: Option<u64>,
}

impl<'a> ReplayCursorV1<'a> {
    /// Validate and create a cursor; no event is exposed until advance_to.
    pub fn new(bundle: &'a AgentObservationBundleV1) -> Result<Self, ContractError> {
        validate_bundle_v1(bundle)?;
        let mut ordered_event_indices: Vec<usize> =
            (0..bundle.transport_events.len()).collect();
        ordered_event_indices.sort_by_key(|index| {
            let event = &bundle.transport_events[*index];
            (event.availability_tick, event.arrival_order)
        });
        Ok(Self {
            bundle,
            ordered_event_indices,
            next_event: 0,
            last_tick: None,
        })
    }

    /// Release all transport events whose availability tick is at most tick.
    ///
    /// Calls must be monotonic. A future event is never returned early.
    pub fn advance_to(
        &mut self,
        tick: u64,
    ) -> Result<Vec<ReplayDeliveryV1<'a>>, ContractError> {
        if self.last_tick.is_some_and(|previous| tick < previous) {
            return Err(ContractError::ReplayTimeRegression);
        }
        self.last_tick = Some(tick);
        let mut ready = Vec::new();
        while let Some(index) = self.ordered_event_indices.get(self.next_event).copied() {
            let event = &self.bundle.transport_events[index];
            if event.availability_tick > tick {
                break;
            }
            let observation = event.observation_id.as_deref().and_then(|id| {
                self.bundle
                    .observations
                    .iter()
                    .find(|item| item.observation_id() == id)
            });
            ready.push(ReplayDeliveryV1 {
                transport: event,
                observation,
            });
            self.next_event += 1;
        }
        Ok(ready)
    }

    /// Number of transport events that have not yet become available.
    pub fn pending_event_count(&self) -> usize {
        self.ordered_event_indices.len().saturating_sub(self.next_event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> AgentObservationBundleV1 {
        serde_json::from_str(include_str!("../fixtures/agent-observations-v1.json"))
            .expect("fixture JSON must parse")
    }

    #[test]
    fn fixture_validates_and_artifact_digests_match() {
        let bundle = fixture();
        validate_bundle_v1(&bundle).unwrap();

        let video = include_bytes!("../fixtures/media/frame.gray8");
        let audio = include_bytes!("../fixtures/media/audio.s16le");
        verify_artifact_bytes_v1(
            bundle.observations[0].artifact_ref(),
            video,
        )
        .unwrap();
        verify_artifact_bytes_v1(
            bundle.observations[1].artifact_ref(),
            audio,
        )
        .unwrap();
    }

    #[test]
    fn unknown_truth_fields_are_rejected() {
        let poisoned = include_str!("../fixtures/agent-observations-v1.json")
            .replace(
                "\"stream_id\":",
                "\"world_truth\": {\"entity_id\": \"hidden-entity\"}, \"stream_id\":",
            );
        assert!(serde_json::from_str::<AgentObservationBundleV1>(&poisoned).is_err());
    }

    #[test]
    fn clock_reset_or_distinct_source_is_incomparable() {
        let camera = ClockDomainV1 {
            domain_id: "camera".to_owned(),
            generation: 4,
            time_scale_id: "monotonic-v1".to_owned(),
        };
        let same = camera.clone();
        let reset = ClockDomainV1 {
            generation: 5,
            ..camera.clone()
        };
        let microphone = ClockDomainV1 {
            domain_id: "microphone".to_owned(),
            ..camera.clone()
        };
        assert_eq!(
            clock_relation_v1(&camera, &same),
            ClockRelationV1::SameClockGeneration
        );
        assert_eq!(
            clock_relation_v1(&camera, &reset),
            ClockRelationV1::Incomparable
        );
        assert_eq!(
            clock_relation_v1(&camera, &microphone),
            ClockRelationV1::Incomparable
        );
    }

    #[test]
    fn unsafe_paths_and_bad_shapes_fail_closed() {
        let mut bundle = fixture();
        if let AgentObservationV1::VideoFrame(frame) = &mut bundle.observations[0] {
            frame.artifact.relative_path = "../truth/secret.json".to_owned();
        }
        assert_eq!(
            validate_bundle_v1(&bundle),
            Err(ContractError::InvalidArtifactPath)
        );

        let mut bundle = fixture();
        if let AgentObservationV1::VideoFrame(frame) = &mut bundle.observations[0] {
            frame.stride_bytes = 1;
        }
        assert_eq!(
            validate_bundle_v1(&bundle),
            Err(ContractError::InvalidVideoShape)
        );

        let mut bundle = fixture();
        if let AgentObservationV1::AudioChunk(chunk) = &mut bundle.observations[1] {
            chunk.source_end_sample = 5;
        }
        assert_eq!(
            validate_bundle_v1(&bundle),
            Err(ContractError::InvalidAudioShape)
        );
    }

    #[test]
    fn mismatched_artifact_digest_or_length_is_rejected() {
        let bundle = fixture();
        let reference = bundle.observations[0].artifact_ref();
        assert_eq!(
            verify_artifact_bytes_v1(&reference, b"ABCE"),
            Err(ContractError::ArtifactIntegrityMismatch)
        );
        assert_eq!(
            verify_artifact_bytes_v1(&reference, b"ABC"),
            Err(ContractError::ArtifactIntegrityMismatch)
        );
    }

    #[test]
    fn replay_never_releases_future_events_and_requires_monotonic_time() {
        let bundle = fixture();
        let mut cursor = ReplayCursorV1::new(&bundle).unwrap();

        assert_eq!(cursor.advance_to(1).unwrap().len(), 0);
        assert_eq!(cursor.pending_event_count(), 3);

        let at_two = cursor.advance_to(2).unwrap();
        assert_eq!(at_two.len(), 1);
        assert_eq!(at_two[0].transport.event_id, "arrival-video-0001");
        assert!(matches!(
            at_two[0].observation,
            Some(AgentObservationV1::VideoFrame(_))
        ));

        let at_three = cursor.advance_to(3).unwrap();
        assert_eq!(at_three.len(), 1);
        assert_eq!(at_three[0].transport.event_id, "arrival-audio-0001");

        let at_four = cursor.advance_to(4).unwrap();
        assert_eq!(at_four.len(), 1);
        assert_eq!(at_four[0].transport.disposition, TransportDispositionV1::Dropped);
        assert!(at_four[0].observation.is_none());
        assert_eq!(cursor.pending_event_count(), 0);

        assert!(matches!(
            cursor.advance_to(3),
            Err(ContractError::ReplayTimeRegression)
        ));
    }

    #[test]
    fn duplicate_and_reordered_events_preserve_first_delivery_semantics() {
        let mut bundle = fixture();
        bundle.transport_events.push(TransportEventV1 {
            event_id: "duplicate-audio-0001".to_owned(),
            sensor_id: "microphone-front".to_owned(),
            source_sequence: 1,
            availability_tick: 5,
            arrival_order: 4,
            disposition: TransportDispositionV1::Duplicate,
            observation_id: Some("obs-audio-0001".to_owned()),
        });
        validate_bundle_v1(&bundle).unwrap();

        let mut reordered = fixture();
        let mut late_chunk = reordered.observations[1].clone();
        if let AgentObservationV1::AudioChunk(chunk) = &mut late_chunk {
            chunk.observation_id = "obs-audio-0000".to_owned();
            chunk.source_sequence = 0;
        }
        reordered.observations.push(late_chunk);
        reordered.transport_events.push(TransportEventV1 {
            event_id: "reordered-audio-0000".to_owned(),
            sensor_id: "microphone-front".to_owned(),
            source_sequence: 0,
            availability_tick: 5,
            arrival_order: 4,
            disposition: TransportDispositionV1::Reordered,
            observation_id: Some("obs-audio-0000".to_owned()),
        });
        validate_bundle_v1(&reordered).unwrap();
    }

    #[test]
    fn missing_first_delivery_and_unexpected_drop_payload_are_rejected() {
        let mut missing = fixture();
        missing.transport_events.retain(|event| event.event_id != "arrival-audio-0001");
        assert_eq!(
            validate_bundle_v1(&missing),
            Err(ContractError::MissingFirstDelivery)
        );

        let mut malformed = fixture();
        malformed.transport_events[2].observation_id = Some("obs-audio-0001".to_owned());
        assert_eq!(
            validate_bundle_v1(&malformed),
            Err(ContractError::UnexpectedObservationReference)
        );
    }
}

