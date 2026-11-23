// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Provenance-bound receipts for derived audiovisual representations.
//!
//! A derived receipt never substitutes for source media. It binds an encoder
//! output to the exact acquisition result, source interval, representation
//! space, payload digest and candidate-visibility boundary.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current derived-representation receipt schema version.
pub const DERIVED_REPRESENTATION_SCHEMA_V1: u32 = 1;

/// Deterministic validation error for derived representation receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivedReceiptError {
    message: String,
}

impl DerivedReceiptError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for DerivedReceiptError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DerivedReceiptError {}

/// Source modality from which an artifact was derived.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModalityV1 {
    /// Audio source media.
    Audio,
    /// Video source media.
    Video,
    /// Synchronized audiovisual source media.
    Audiovisual,
}

/// Semantic kind of a derived representation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationKindV1 {
    /// Deterministic signal-processing measurements such as F0, RMS or energy.
    DspFeatures,
    /// Hyperdimensional representation such as HV16 or ContinuousHV.
    HdcEmbedding,
    /// Learned vector representation from a pretrained or trained encoder.
    LearnedEmbedding,
    /// Word/phone transcript alignment with source-time spans.
    TranscriptAlignment,
    /// ASR hypothesis or token sequence derived from audio.
    AsrHypothesis,
    /// Object, action or region descriptors derived from video.
    VisualFeatures,
}

/// Immutable source identity and clock generation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceIdentityV1 {
    /// Stable source/media identifier.
    pub source_id: String,
    /// SHA-256 digest of the immutable source object or source segment.
    pub source_content_sha256: String,
    /// Clock domain used by source timestamps/sample indices.
    pub clock_domain_id: String,
    /// Clock generation; increments after a reset/reconfiguration.
    pub clock_generation: u64,
}

/// Exact audio sample span. End sample is exclusive.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AudioSampleSpanV1 {
    /// First source sample, inclusive.
    pub start_sample: u64,
    /// End source sample, exclusive.
    pub end_sample: u64,
    /// Samples per second.
    pub sample_rate_hz: u32,
    /// Number of interleaved source channels.
    pub channels: u16,
}

/// Exact video frame span. End frame is exclusive.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VideoFrameSpanV1 {
    /// First source frame index, inclusive.
    pub start_frame: u64,
    /// End source frame index, exclusive.
    pub end_frame: u64,
    /// Rational source frame rate numerator.
    pub fps_numerator: u32,
    /// Rational source frame rate denominator.
    pub fps_denominator: u32,
}

/// Exact source interval from which a derived artifact was computed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "span_kind", content = "span", rename_all = "snake_case")]
pub enum SourceSpanV1 {
    /// Audio sample interval.
    AudioSamples(AudioSampleSpanV1),
    /// Video frame interval.
    VideoFrames(VideoFrameSpanV1),
    /// Synchronized audio and video intervals.
    Audiovisual {
        /// Audio interval.
        audio: AudioSampleSpanV1,
        /// Video interval.
        video: VideoFrameSpanV1,
    },
}

/// Outcome status of the parent acquisition.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParentAcquisitionStatusV1 {
    /// Complete parent observation delivered.
    Delivered,
    /// Valid strict subset of the requested parent observation delivered.
    Partial,
    /// Parent acquisition failed.
    Failed,
    /// Parent source/evidence was unavailable.
    Unavailable,
    /// Parent acquisition was cancelled.
    Cancelled,
    /// Parent result missed its decision deadline.
    Late,
}

/// Evaluator-attested facts about the parent acquisition result.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParentAcquisitionEvidenceV1 {
    /// Executor request ID from the accepted acquisition.
    pub request_id: String,
    /// SHA-256 digest of the executor result event.
    pub result_event_sha256: String,
    /// Immutable parent source identity.
    pub source: SourceIdentityV1,
    /// Actual source intervals delivered by the executor.
    pub delivered_spans: Vec<SourceSpanV1>,
    /// Actual parent acquisition outcome.
    pub status: ParentAcquisitionStatusV1,
    /// Logical tick when the parent result entered the evidence ledger.
    pub result_tick: u64,
}

/// Executable identity of the encoder or transformation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EncoderIdentityV1 {
    /// Stable encoder name, e.g. direct-mel-hv16.
    pub encoder_id: String,
    /// Encoder semantic/version identifier.
    pub encoder_version: String,
    /// SHA-256 digest of the encoder source/build artifact.
    pub build_sha256: String,
    /// SHA-256 digest of the canonical frontend/configuration.
    pub config_sha256: String,
    /// Stable basis/seed identity when the representation uses one.
    pub basis_seed_identity: Option<String>,
    /// SHA-256 digest of deterministic initialization state, when applicable.
    pub initialization_sha256: Option<String>,
}

/// Numeric element type of one feature value.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureElementTypeV1 {
    /// IEEE-754 binary32 little-endian.
    F32Le,
    /// IEEE-754 binary16 little-endian.
    F16Le,
    /// Signed 8-bit integer.
    I8,
    /// Unsigned 8-bit integer.
    U8,
    /// Packed one-bit values.
    PackedBits,
}

/// Memory layout of a feature tensor.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureLayoutV1 {
    /// Last dimension varies fastest.
    RowMajor,
    /// First dimension varies fastest.
    ColumnMajor,
    /// Time-major sequence layout.
    TimeMajor,
    /// Packed bitstream; bit order is declared by quantization metadata.
    PackedBitstream,
}

/// Declared normalization applied to feature values.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizationV1 {
    /// No normalization.
    None,
    /// L2 unit-norm normalization.
    L2,
    /// Per-feature z-score normalization.
    #[serde(rename = "per_feature_z_score")]
    PerFeatureZScore,
    /// Values scaled to the unit interval.
    UnitInterval,
}

/// Quantization or packing metadata. Scales use integer nano-units.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "quantization_kind", rename_all = "snake_case")]
pub enum QuantizationV1 {
    /// No quantization.
    None,
    /// Symmetric signed uniform quantization.
    SymmetricUniform {
        /// Number of quantized bits.
        bit_width: u8,
        /// Quantization scale multiplied by one billion.
        scale_nano: u64,
    },
    /// Affine uniform quantization.
    AffineUniform {
        /// Number of quantized bits.
        bit_width: u8,
        /// Quantization scale multiplied by one billion.
        scale_nano: u64,
        /// Integer zero point.
        zero_point: i64,
    },
    /// Binary hypervector packing.
    Binary {
        /// Bit order within each byte.
        bit_order: BitOrderV1,
    },
}

/// Bit order used by packed binary representations.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BitOrderV1 {
    /// Most-significant bit first.
    MostSignificantFirst,
    /// Least-significant bit first.
    LeastSignificantFirst,
}

/// Complete feature-space declaration.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeatureDescriptorV1 {
    /// Tensor dimensions; every dimension must be nonzero.
    pub shape: Vec<u64>,
    /// Tensor memory layout.
    pub layout: FeatureLayoutV1,
    /// Numeric element type.
    pub element_type: FeatureElementTypeV1,
    /// Declared normalization.
    pub normalization: NormalizationV1,
    /// Quantization or binary packing declaration.
    pub quantization: QuantizationV1,
    /// Exact encoded feature payload length in bytes.
    pub byte_length: u64,
}

/// Re-encoding/replay comparison mode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ReproducibilityModeV1 {
    /// Encoded payload must be bit-identical.
    BitExact,
    /// Floating-point re-encoding may differ within an absolute tolerance.
    F32AbsoluteTolerance {
        /// Absolute tolerance in parts per million.
        tolerance_ppm: u32,
    },
}

/// Candidate visibility boundary for a derived artifact.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "visibility", rename_all = "snake_case")]
pub enum VisibilityV1 {
    /// Artifact was delivered to the candidate transcript at this tick.
    CandidateVisible {
        /// Logical candidate-visible delivery tick.
        delivered_tick: u64,
    },
    /// Artifact is retained only in the evaluator plane.
    EvaluatorOnly {
        /// Logical evaluator storage tick.
        sealed_tick: u64,
    },
}

/// Versioned provenance receipt for one derived audiovisual artifact.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DerivedRepresentationReceiptV1 {
    /// Receipt schema version.
    pub schema_version: u32,
    /// Stable derived artifact ID.
    pub artifact_id: String,
    /// Source modality.
    pub modality: ModalityV1,
    /// Semantic representation kind.
    pub representation_kind: RepresentationKindV1,
    /// Encoder and transform identity.
    pub encoder: EncoderIdentityV1,
    /// Immutable source identity.
    pub source: SourceIdentityV1,
    /// Exact source interval consumed by this derivation.
    pub source_span: SourceSpanV1,
    /// Parent executor request ID.
    pub parent_acquisition_request_id: String,
    /// SHA-256 digest of the exact parent acquisition result event.
    pub parent_acquisition_result_sha256: String,
    /// Exact feature-space description.
    pub features: FeatureDescriptorV1,
    /// SHA-256 digest of the exact derived payload bytes.
    pub artifact_content_sha256: String,
    /// Logical tick when derivation completed.
    pub created_tick: u64,
    /// Candidate/evaluator visibility record.
    pub visibility: VisibilityV1,
    /// Candidate transcript head at delivery, absent for evaluator-only artifacts.
    pub transcript_parent_head_sha256: Option<String>,
    /// Causal parent artifact IDs; duplicates and self-reference are invalid.
    pub causal_parent_artifact_ids: Vec<String>,
    /// Declared re-encoding comparison mode.
    pub reproducibility: ReproducibilityModeV1,
}

/// Compute lowercase SHA-256 hex for exact payload bytes.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// Validate source-span structure and modality agreement.
pub fn validate_source_span(
    modality: ModalityV1,
    span: &SourceSpanV1,
) -> Result<(), DerivedReceiptError> {
    match (modality, span) {
        (ModalityV1::Audio, SourceSpanV1::AudioSamples(audio)) => validate_audio_span(audio),
        (ModalityV1::Video, SourceSpanV1::VideoFrames(video)) => validate_video_span(video),
        (ModalityV1::Audiovisual, SourceSpanV1::Audiovisual { audio, video }) => {
            validate_audio_span(audio)?;
            validate_video_span(video)
        }
        _ => Err(DerivedReceiptError::new(
            "source span kind does not match artifact modality",
        )),
    }
}

/// Validate feature dimensions, encoded size and quantization metadata.
pub fn validate_feature_descriptor(
    features: &FeatureDescriptorV1,
) -> Result<(), DerivedReceiptError> {
    if features.shape.is_empty() || features.shape.iter().any(|dimension| *dimension == 0) {
        return Err(DerivedReceiptError::new(
            "feature shape must contain only nonzero dimensions",
        ));
    }

    let element_count = features
        .shape
        .iter()
        .try_fold(1_u64, |product, dimension| product.checked_mul(*dimension))
        .ok_or_else(|| DerivedReceiptError::new("feature element count overflow"))?;

    let expected_bytes = match features.element_type {
        FeatureElementTypeV1::F32Le => element_count.checked_mul(4),
        FeatureElementTypeV1::F16Le => element_count.checked_mul(2),
        FeatureElementTypeV1::I8 | FeatureElementTypeV1::U8 => Some(element_count),
        FeatureElementTypeV1::PackedBits => element_count.checked_add(7).map(|bits| bits / 8),
    }
    .ok_or_else(|| DerivedReceiptError::new("feature byte length overflow"))?;

    if expected_bytes != features.byte_length {
        return Err(DerivedReceiptError::new(format!(
            "feature byte length {} does not match shape/type requirement {expected_bytes}",
            features.byte_length
        )));
    }

    match (features.element_type, features.quantization) {
        (FeatureElementTypeV1::F32Le | FeatureElementTypeV1::F16Le, QuantizationV1::None) => {}
        (
            FeatureElementTypeV1::I8,
            QuantizationV1::SymmetricUniform {
                bit_width,
                scale_nano,
            },
        ) => validate_quantization(bit_width, scale_nano)?,
        (
            FeatureElementTypeV1::I8 | FeatureElementTypeV1::U8,
            QuantizationV1::AffineUniform {
                bit_width,
                scale_nano,
                ..
            },
        ) => validate_quantization(bit_width, scale_nano)?,
        (FeatureElementTypeV1::PackedBits, QuantizationV1::Binary { .. }) => {
            if features.layout != FeatureLayoutV1::PackedBitstream {
                return Err(DerivedReceiptError::new(
                    "binary representation requires packed_bitstream layout",
                ));
            }
        }
        _ => {
            return Err(DerivedReceiptError::new(
                "quantization metadata is incompatible with feature element type",
            ));
        }
    }

    if features.element_type == FeatureElementTypeV1::PackedBits
        && features.layout != FeatureLayoutV1::PackedBitstream
    {
        return Err(DerivedReceiptError::new(
            "packed bits require packed_bitstream layout",
        ));
    }
    if features.element_type != FeatureElementTypeV1::PackedBits
        && features.layout == FeatureLayoutV1::PackedBitstream
    {
        return Err(DerivedReceiptError::new(
            "packed_bitstream layout requires packed bit elements",
        ));
    }
    Ok(())
}

/// Validate a receipt against evaluator-attested parent facts, exact payload
/// bytes and the candidate decision cutoff.
pub fn validate_derived_receipt(
    receipt: &DerivedRepresentationReceiptV1,
    parent: &ParentAcquisitionEvidenceV1,
    payload: &[u8],
    decision_cutoff_tick: u64,
) -> Result<(), DerivedReceiptError> {
    if receipt.schema_version != DERIVED_REPRESENTATION_SCHEMA_V1 {
        return Err(DerivedReceiptError::new(
            "unsupported receipt schema version",
        ));
    }
    require_text(&receipt.artifact_id, "artifact_id")?;
    require_text(&receipt.encoder.encoder_id, "encoder_id")?;
    require_text(&receipt.encoder.encoder_version, "encoder_version")?;
    require_text(&receipt.source.source_id, "source_id")?;
    require_text(&receipt.source.clock_domain_id, "clock_domain_id")?;
    require_text(
        &receipt.parent_acquisition_request_id,
        "parent_acquisition_request_id",
    )?;
    validate_digest(
        &receipt.source.source_content_sha256,
        "source_content_sha256",
    )?;
    validate_digest(&receipt.encoder.build_sha256, "encoder.build_sha256")?;
    validate_digest(&receipt.encoder.config_sha256, "encoder.config_sha256")?;
    if let Some(digest) = &receipt.encoder.initialization_sha256 {
        validate_digest(digest, "encoder.initialization_sha256")?;
    }
    validate_digest(
        &receipt.parent_acquisition_result_sha256,
        "parent_acquisition_result_sha256",
    )?;
    validate_digest(&receipt.artifact_content_sha256, "artifact_content_sha256")?;
    validate_source_span(receipt.modality, &receipt.source_span)?;
    validate_feature_descriptor(&receipt.features)?;
    validate_reproducibility(receipt.reproducibility, receipt.features.element_type)?;

    if receipt.representation_kind == RepresentationKindV1::TranscriptAlignment
        || receipt.representation_kind == RepresentationKindV1::AsrHypothesis
    {
        if receipt.modality != ModalityV1::Audio {
            return Err(DerivedReceiptError::new(
                "transcript alignment and ASR hypotheses require audio modality",
            ));
        }
    }
    if receipt.representation_kind == RepresentationKindV1::VisualFeatures
        && receipt.modality == ModalityV1::Audio
    {
        return Err(DerivedReceiptError::new(
            "visual features require video or audiovisual modality",
        ));
    }
    if receipt.representation_kind == RepresentationKindV1::HdcEmbedding {
        require_text(
            receipt.encoder.basis_seed_identity.as_deref().unwrap_or(""),
            "encoder.basis_seed_identity",
        )?;
        let initialization = receipt
            .encoder
            .initialization_sha256
            .as_deref()
            .ok_or_else(|| DerivedReceiptError::new("HDC requires initialization_sha256"))?;
        validate_digest(initialization, "encoder.initialization_sha256")?;
    }

    if receipt.source != parent.source {
        return Err(DerivedReceiptError::new(
            "derived source identity differs from parent acquisition source",
        ));
    }
    if receipt.parent_acquisition_request_id != parent.request_id {
        return Err(DerivedReceiptError::new(
            "derived artifact parent request ID mismatch",
        ));
    }
    if receipt.parent_acquisition_result_sha256 != parent.result_event_sha256 {
        return Err(DerivedReceiptError::new(
            "derived artifact parent result digest mismatch",
        ));
    }
    if !matches!(
        parent.status,
        ParentAcquisitionStatusV1::Delivered | ParentAcquisitionStatusV1::Partial
    ) {
        return Err(DerivedReceiptError::new(
            "cannot derive candidate evidence from a failed, unavailable, cancelled or late acquisition",
        ));
    }
    if !parent
        .delivered_spans
        .iter()
        .any(|delivered| span_is_covered_by(&receipt.source_span, delivered))
    {
        return Err(DerivedReceiptError::new(
            "derived source interval is not contained in an actual delivered parent interval",
        ));
    }
    if receipt.created_tick < parent.result_tick {
        return Err(DerivedReceiptError::new(
            "derived artifact creation predates parent acquisition result",
        ));
    }
    if receipt.features.byte_length != payload.len() as u64 {
        return Err(DerivedReceiptError::new(
            "feature descriptor byte length differs from delivered payload length",
        ));
    }
    if receipt.artifact_content_sha256 != sha256_hex(payload) {
        return Err(DerivedReceiptError::new(
            "derived artifact content digest mismatch",
        ));
    }

    let mut unique_parents = BTreeSet::new();
    for parent_id in &receipt.causal_parent_artifact_ids {
        require_text(parent_id, "causal_parent_artifact_id")?;
        if parent_id == &receipt.artifact_id || !unique_parents.insert(parent_id) {
            return Err(DerivedReceiptError::new(
                "causal parent artifact IDs must be unique and cannot self-reference",
            ));
        }
    }

    match receipt.visibility {
        VisibilityV1::CandidateVisible { delivered_tick } => {
            if delivered_tick < receipt.created_tick {
                return Err(DerivedReceiptError::new(
                    "candidate-visible delivery predates artifact creation",
                ));
            }
            if delivered_tick > decision_cutoff_tick {
                return Err(DerivedReceiptError::new(
                    "candidate-visible artifact arrived after the decision cutoff",
                ));
            }
            let transcript_head = receipt
                .transcript_parent_head_sha256
                .as_deref()
                .ok_or_else(|| {
                    DerivedReceiptError::new(
                        "candidate-visible artifact requires a transcript parent head",
                    )
                })?;
            validate_digest(transcript_head, "transcript_parent_head_sha256")?;
        }
        VisibilityV1::EvaluatorOnly { sealed_tick } => {
            if receipt.transcript_parent_head_sha256.is_some() {
                return Err(DerivedReceiptError::new(
                    "evaluator-only artifact cannot carry a candidate transcript parent head",
                ));
            }
            if sealed_tick < receipt.created_tick {
                return Err(DerivedReceiptError::new(
                    "evaluator seal predates artifact creation",
                ));
            }
        }
    }
    Ok(())
}

/// Require an artifact to have been candidate-visible before the decision cutoff.
pub fn require_candidate_visible(
    receipt: &DerivedRepresentationReceiptV1,
    decision_cutoff_tick: u64,
) -> Result<u64, DerivedReceiptError> {
    match receipt.visibility {
        VisibilityV1::CandidateVisible { delivered_tick }
            if delivered_tick <= decision_cutoff_tick =>
        {
            Ok(delivered_tick)
        }
        VisibilityV1::CandidateVisible { .. } => Err(DerivedReceiptError::new(
            "candidate-visible artifact is post-cutoff",
        )),
        VisibilityV1::EvaluatorOnly { .. } => Err(DerivedReceiptError::new(
            "evaluator-only artifact is not candidate evidence",
        )),
    }
}

/// Reject comparisons between incompatible representation spaces.
pub fn require_compatible_representation(
    left: &DerivedRepresentationReceiptV1,
    right: &DerivedRepresentationReceiptV1,
) -> Result<(), DerivedReceiptError> {
    if left.modality != right.modality
        || left.representation_kind != right.representation_kind
        || left.encoder != right.encoder
        || left.features.shape != right.features.shape
        || left.features.layout != right.features.layout
        || left.features.element_type != right.features.element_type
        || left.features.normalization != right.features.normalization
        || left.features.quantization != right.features.quantization
    {
        return Err(DerivedReceiptError::new(
            "derived artifacts belong to incompatible representation spaces",
        ));
    }
    Ok(())
}

/// Compute a stable representation-space digest, excluding source and payload identity.
pub fn representation_space_sha256(
    receipt: &DerivedRepresentationReceiptV1,
) -> Result<String, DerivedReceiptError> {
    #[derive(Serialize)]
    struct SpaceMaterial<'a> {
        modality: ModalityV1,
        representation_kind: RepresentationKindV1,
        encoder: &'a EncoderIdentityV1,
        features: &'a FeatureDescriptorV1,
    }

    let material = SpaceMaterial {
        modality: receipt.modality,
        representation_kind: receipt.representation_kind,
        encoder: &receipt.encoder,
        features: &receipt.features,
    };
    let canonical = serde_json::to_vec(&material).map_err(|error| {
        DerivedReceiptError::new(format!(
            "representation space serialization failed: {error}"
        ))
    })?;
    Ok(sha256_hex(&canonical))
}

fn validate_audio_span(span: &AudioSampleSpanV1) -> Result<(), DerivedReceiptError> {
    if span.end_sample <= span.start_sample || span.sample_rate_hz == 0 || span.channels == 0 {
        return Err(DerivedReceiptError::new("invalid audio sample span"));
    }
    Ok(())
}

fn validate_video_span(span: &VideoFrameSpanV1) -> Result<(), DerivedReceiptError> {
    if span.end_frame <= span.start_frame || span.fps_numerator == 0 || span.fps_denominator == 0 {
        return Err(DerivedReceiptError::new("invalid video frame span"));
    }
    Ok(())
}

fn validate_reproducibility(
    mode: ReproducibilityModeV1,
    element_type: FeatureElementTypeV1,
) -> Result<(), DerivedReceiptError> {
    if let ReproducibilityModeV1::F32AbsoluteTolerance { tolerance_ppm } = mode {
        if !matches!(
            element_type,
            FeatureElementTypeV1::F32Le | FeatureElementTypeV1::F16Le
        ) {
            return Err(DerivedReceiptError::new(
                "floating-point tolerance is only valid for floating-point payloads",
            ));
        }
        if tolerance_ppm > 10_000 {
            return Err(DerivedReceiptError::new(
                "floating-point replay tolerance exceeds 10,000 ppm",
            ));
        }
    }
    Ok(())
}

fn validate_quantization(bit_width: u8, scale_nano: u64) -> Result<(), DerivedReceiptError> {
    if !(2..=8).contains(&bit_width) || scale_nano == 0 {
        return Err(DerivedReceiptError::new(
            "8-bit quantized elements require bit width 2..=8 and nonzero scale",
        ));
    }
    Ok(())
}

fn validate_digest(value: &str, field: &str) -> Result<(), DerivedReceiptError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(DerivedReceiptError::new(format!(
            "{field} must be a 64-character SHA-256 hex digest"
        )));
    }
    Ok(())
}

fn require_text(value: &str, field: &str) -> Result<(), DerivedReceiptError> {
    if value.trim().is_empty() {
        return Err(DerivedReceiptError::new(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn span_is_covered_by(derived: &SourceSpanV1, delivered: &SourceSpanV1) -> bool {
    match (derived, delivered) {
        (SourceSpanV1::AudioSamples(a), SourceSpanV1::AudioSamples(b)) => {
            a.sample_rate_hz == b.sample_rate_hz
                && a.channels == b.channels
                && a.start_sample >= b.start_sample
                && a.end_sample <= b.end_sample
        }
        (SourceSpanV1::VideoFrames(a), SourceSpanV1::VideoFrames(b)) => {
            a.fps_numerator == b.fps_numerator
                && a.fps_denominator == b.fps_denominator
                && a.start_frame >= b.start_frame
                && a.end_frame <= b.end_frame
        }
        (
            SourceSpanV1::Audiovisual {
                audio: da,
                video: dv,
            },
            SourceSpanV1::Audiovisual {
                audio: pa,
                video: pv,
            },
        ) => {
            span_is_covered_by(
                &SourceSpanV1::AudioSamples(*da),
                &SourceSpanV1::AudioSamples(*pa),
            ) && span_is_covered_by(
                &SourceSpanV1::VideoFrames(*dv),
                &SourceSpanV1::VideoFrames(*pv),
            )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: &[u8]) -> String {
        sha256_hex(value)
    }

    fn source() -> SourceIdentityV1 {
        SourceIdentityV1 {
            source_id: "mic-01".to_string(),
            source_content_sha256: digest(b"source-media"),
            clock_domain_id: "device-clock-01".to_string(),
            clock_generation: 4,
        }
    }

    fn source_span() -> SourceSpanV1 {
        SourceSpanV1::AudioSamples(AudioSampleSpanV1 {
            start_sample: 1_000,
            end_sample: 2_000,
            sample_rate_hz: 16_000,
            channels: 1,
        })
    }

    fn parent(status: ParentAcquisitionStatusV1) -> ParentAcquisitionEvidenceV1 {
        ParentAcquisitionEvidenceV1 {
            request_id: "request-a1".to_string(),
            result_event_sha256: digest(b"acquisition-result"),
            source: source(),
            delivered_spans: vec![SourceSpanV1::AudioSamples(AudioSampleSpanV1 {
                start_sample: 0,
                end_sample: 4_000,
                sample_rate_hz: 16_000,
                channels: 1,
            })],
            status,
            result_tick: 10,
        }
    }

    fn receipt(payload: &[u8]) -> DerivedRepresentationReceiptV1 {
        DerivedRepresentationReceiptV1 {
            schema_version: DERIVED_REPRESENTATION_SCHEMA_V1,
            artifact_id: "artifact-hv16-0001".to_string(),
            modality: ModalityV1::Audio,
            representation_kind: RepresentationKindV1::HdcEmbedding,
            encoder: EncoderIdentityV1 {
                encoder_id: "direct-mel-hv16".to_string(),
                encoder_version: "DirectMelFrameV1".to_string(),
                build_sha256: digest(b"encoder-build"),
                config_sha256: digest(b"frontend-config"),
                basis_seed_identity: Some("symthaea-hv16-basis-v1".to_string()),
                initialization_sha256: Some(digest(b"basis-initialization")),
            },
            source: source(),
            source_span: source_span(),
            parent_acquisition_request_id: "request-a1".to_string(),
            parent_acquisition_result_sha256: digest(b"acquisition-result"),
            features: FeatureDescriptorV1 {
                shape: vec![2_048],
                layout: FeatureLayoutV1::PackedBitstream,
                element_type: FeatureElementTypeV1::PackedBits,
                normalization: NormalizationV1::None,
                quantization: QuantizationV1::Binary {
                    bit_order: BitOrderV1::MostSignificantFirst,
                },
                byte_length: payload.len() as u64,
            },
            artifact_content_sha256: digest(payload),
            created_tick: 11,
            visibility: VisibilityV1::CandidateVisible { delivered_tick: 12 },
            transcript_parent_head_sha256: Some(digest(b"transcript-head")),
            causal_parent_artifact_ids: vec!["raw-audio-window-01".to_string()],
            reproducibility: ReproducibilityModeV1::BitExact,
        }
    }

    #[test]
    fn valid_derived_receipt_replays_against_exact_parent_and_payload() {
        let payload = vec![0xA5; 256];
        assert!(
            validate_derived_receipt(
                &receipt(&payload),
                &parent(ParentAcquisitionStatusV1::Delivered),
                &payload,
                20
            )
            .is_ok()
        );
    }

    #[test]
    fn payload_mutation_is_rejected() {
        let payload = vec![0xA5; 256];
        let receipt = receipt(&payload);
        let mut mutated = payload.clone();
        mutated[7] ^= 0x01;
        assert!(
            validate_derived_receipt(
                &receipt,
                &parent(ParentAcquisitionStatusV1::Delivered),
                &mutated,
                20
            )
            .is_err()
        );
    }

    #[test]
    fn source_digest_or_clock_generation_mutation_is_rejected() {
        let payload = vec![0xA5; 256];
        let mut changed = receipt(&payload);
        changed.source.source_content_sha256 = digest(b"other-source");
        assert!(
            validate_derived_receipt(
                &changed,
                &parent(ParentAcquisitionStatusV1::Delivered),
                &payload,
                20
            )
            .is_err()
        );

        let mut changed = receipt(&payload);
        changed.source.clock_generation += 1;
        assert!(validate_derived_receipt(
            &changed,
            &parent(ParentAcquisitionStatusV1::Delivered),
            &payload,
            20
        ).is_err());
    }

    #[test]
    fn parent_request_or_result_mutation_is_rejected() {
        let payload = vec![0xA5; 256];
        let receipt = receipt(&payload);
        let mut parent = parent(ParentAcquisitionStatusV1::Delivered);
        parent.request_id = "different-request".to_string();
        assert!(validate_derived_receipt(&receipt, &parent, &payload, 20).is_err());

        let mut parent = parent(ParentAcquisitionStatusV1::Delivered);
        parent.result_event_sha256 = digest(b"different-result");
        assert!(validate_derived_receipt(&receipt, &parent, &payload, 20).is_err());
    }

    #[test]
    fn unavailable_or_failed_parent_cannot_mint_derived_candidate_evidence() {
        let payload = vec![0xA5; 256];
        let receipt = receipt(&payload);
        for status in [
            ParentAcquisitionStatusV1::Failed,
            ParentAcquisitionStatusV1::Unavailable,
            ParentAcquisitionStatusV1::Cancelled,
            ParentAcquisitionStatusV1::Late,
        ] {
            assert!(validate_derived_receipt(&receipt, &parent(status), &payload, 20).is_err());
        }
    }

    #[test]
    fn derived_interval_must_be_covered_by_actual_parent_delivery() {
        let payload = vec![0xA5; 256];
        let mut receipt = receipt(&payload);
        if let SourceSpanV1::AudioSamples(span) = &mut receipt.source_span {
            span.start_sample = 3_900;
            span.end_sample = 4_100;
        }
        assert!(
            validate_derived_receipt(
                &receipt,
                &parent(ParentAcquisitionStatusV1::Partial),
                &payload,
                20
            )
            .is_err()
        );
    }

    #[test]
    fn delivery_after_decision_cutoff_is_rejected() {
        let payload = vec![0xA5; 256];
        let mut receipt = receipt(&payload);
        receipt.visibility = VisibilityV1::CandidateVisible { delivered_tick: 21 };
        assert!(
            validate_derived_receipt(
                &receipt,
                &parent(ParentAcquisitionStatusV1::Delivered),
                &payload,
                20
            )
            .is_err()
        );
    }

    #[test]
    fn evaluator_only_artifact_cannot_become_candidate_evidence() {
        let payload = vec![0xA5; 256];
        let mut receipt = receipt(&payload);
        receipt.visibility = VisibilityV1::EvaluatorOnly { sealed_tick: 12 };
        receipt.transcript_parent_head_sha256 = None;
        assert!(
            validate_derived_receipt(
                &receipt,
                &parent(ParentAcquisitionStatusV1::Delivered),
                &payload,
                20
            )
            .is_ok()
        );
        assert!(require_candidate_visible(&receipt, 20).is_err());
    }

    #[test]
    fn transcript_parent_is_required_for_candidate_visible_artifacts() {
        let payload = vec![0xA5; 256];
        let mut receipt = receipt(&payload);
        receipt.transcript_parent_head_sha256 = None;
        assert!(validate_derived_receipt(
            &receipt,
            &parent(ParentAcquisitionStatusV1::Delivered),
            &payload,
            20
        ).is_err());
    }

    #[test]
    fn encoder_identity_mutation_changes_representation_space() {
        let payload = vec![0xA5; 256];
        let first = receipt(&payload);
        let mut second = receipt(&payload);
        second.encoder.encoder_version = "DirectMelFrameV2".to_string();
        assert_ne!(
            representation_space_sha256(&first).unwrap(),
            representation_space_sha256(&second).unwrap()
        );
        assert!(require_compatible_representation(&first, &second).is_err());
    }

    #[test]
    fn same_dimension_different_quantization_is_not_comparable() {
        let payload = vec![0xA5; 256];
        let first = receipt(&payload);
        let mut second = receipt(&payload);
        second.features.quantization = QuantizationV1::Binary {
            bit_order: BitOrderV1::LeastSignificantFirst,
        };
        assert!(require_compatible_representation(&first, &second).is_err());
    }

    #[test]
    fn invalid_shape_or_quantization_metadata_is_rejected() {
        let mut features = FeatureDescriptorV1 {
            shape: vec![0, 2_048],
            layout: FeatureLayoutV1::PackedBitstream,
            element_type: FeatureElementTypeV1::PackedBits,
            normalization: NormalizationV1::None,
            quantization: QuantizationV1::Binary {
                bit_order: BitOrderV1::MostSignificantFirst,
            },
            byte_length: 256,
        };
        assert!(validate_feature_descriptor(&features).is_err());

        features.shape = vec![2_048];
        features.layout = FeatureLayoutV1::RowMajor;
        assert!(validate_feature_descriptor(&features).is_err());
    }

    #[test]
    fn audiovisual_receipt_requires_both_exact_modalities() {
        let payload = vec![0_u8; 256];
        let mut receipt = receipt(&payload);
        receipt.modality = ModalityV1::Audiovisual;
        assert!(validate_source_span(receipt.modality, &receipt.source_span).is_err());
        receipt.source_span = SourceSpanV1::Audiovisual {
            audio: AudioSampleSpanV1 {
                start_sample: 1_000,
                end_sample: 2_000,
                sample_rate_hz: 16_000,
                channels: 1,
            },
            video: VideoFrameSpanV1 {
                start_frame: 10,
                end_frame: 20,
                fps_numerator: 30,
                fps_denominator: 1,
            },
        };
        assert!(validate_source_span(receipt.modality, &receipt.source_span).is_ok());
    }

    #[test]
    fn candidate_visibility_requires_created_before_delivery() {
        let payload = vec![0xA5; 256];
        let mut receipt = receipt(&payload);
        receipt.created_tick = 13;
        receipt.visibility = VisibilityV1::CandidateVisible { delivered_tick: 12 };
        assert!(validate_derived_receipt(
            &receipt,
            &parent(ParentAcquisitionStatusV1::Delivered),
            &payload,
            20
        ).is_err());
    }

    #[test]
    fn normalization_variant_uses_schema_stable_snake_case() {
        let value = serde_json::to_value(NormalizationV1::PerFeatureZScore).unwrap();
        assert_eq!(
            value,
            serde_json::Value::String("per_feature_z_score".to_string())
        );
    }

    #[test]
    fn packed_dimension_and_byte_length_are_not_interchangeable() {
        let payload = vec![0xA5; 255];
        let mut receipt = receipt(&vec![0xA5; 256]);
        receipt.features.byte_length = 255;
        receipt.artifact_content_sha256 = digest(&payload);
        assert!(validate_derived_receipt(
            &receipt,
            &parent(ParentAcquisitionStatusV1::Delivered),
            &payload,
            20
        ).is_err());
    }
}
