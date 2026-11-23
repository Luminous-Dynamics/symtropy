# AV Derived Representation Receipt v0

**Issue:** Symtropy #6930  
**Stacks on:** #6897 / draft PR #6897  
**Composes:** #6895 multi-turn trajectory, #6859 claim-level certificates, #6564 acquisition executor, Symthaea #12872 live/offline HDC identity.

## Objective

Bind every derived audiovisual artifact to the exact source observation and acquisition result from which it was computed. Keep source media, DSP measurements, HDC/learned embeddings, transcript alignments and evaluator-only labels distinct.

This crate is a protocol/evaluator contract. It does not implement an encoder, ASR model, scheduler, sensor driver, acquisition executor or claim scorer.

## Key invariants

1. **Source is not representation.** Source media identity and exact source interval are carried separately from the derived payload digest and feature descriptor.
2. **The parent must exist.** A receipt must reference the exact accepted request ID and result-event digest from a delivered or partial parent acquisition.
3. **A derivation cannot widen its source.** Its audio sample/video frame span must be contained in an interval the executor actually delivered, with source clock generation and source identity unchanged.
4. **Feature spaces are typed.** Modality, representation kind, encoder/build/config identity, basis seed, initialization identity, shape, layout, element type, normalization and quantization are part of the representation space.
5. **Visibility is causal.** Candidate-visible artifacts must be created after the parent acquisition result, delivered after creation, and delivered no later than the decision cutoff. Evaluator-only artifacts cannot be presented as candidate evidence.
6. **Payload is exact.** Byte length must match the declared shape and element type, and SHA-256 must match the exact payload bytes.
7. **No accidental vector pooling.** Same-dimensional vectors are not comparable when encoder identity, configuration, basis, modality or quantization differ.

## Rust contract

- `src/lib.rs`: typed, versioned receipt and validation API.
- `schemas/av-derived-representation-receipt-v1.schema.json`: machine-readable JSON Schema.
- The crate is standalone and intentionally outside the repository's general Cargo workspace.

### Representation kinds

- `dsp_features`: F0, RMS, voicing, energy, pause and event measurements.
- `hdc_embedding`: compact HV16 or dense ContinuousHV representations.
- `learned_embedding`: pretrained or task-specific learned feature vectors.
- `transcript_alignment`: word/phone spans tied to source intervals.
- `asr_hypothesis`: transcript or token hypotheses, not independent acoustic evidence.
- `visual_features`: visual descriptors and object/action features.

### Audio and video span handling

Audio intervals use exact source sample indices, sample rate and source channel count. Video intervals use exact frame indices and rational source FPS. An audiovisual representation must name both exact spans. Clock domain and generation are inherited from the parent source identity; resets cannot silently preserve an old receipt.

### Integration status and video boundary

The current #6897 lifecycle adapter is implemented for **audio-window actions**: it calls the canonical forecast/acceptance/result validator, then derives the parent source sample interval from the executor's delivered-audio receipt.

A matching video parent adapter is intentionally not inferred yet. #6897's delivered frame receipt currently carries sequence/frame index and capture timestamp, but it does not yet make the source-native frame index or a source-frame clock mapping explicit enough to prove a derived source-frame interval without ambiguity. Until that envelope is added, a video caller must supply evaluator-attested parent evidence with a genuinely source-native frame mapping; do not relabel request-local frame sequence as source frame index. Audiovisual parents require both independently validated spans.

This is a deliberate fail-closed boundary, not a claim that video derivation integration is complete.

## Reproducibility

Bit-exact replay is the default. Floating-point encoders may explicitly declare an absolute tolerance in integer ppm units. The receipt records the policy but does not claim that a replay was run; re-encoder verification must create a separate measured result.

## Test coverage

The unit suite includes positive replay and adversarial controls for payload mutation, source digest/clock-generation mutation, parent request/result identity mutation, failed/unavailable/cancelled/late acquisitions, source-span widening, post-cutoff visibility, evaluator-only leakage, incompatible same-dimension vector spaces, and feature shape/byte-length mismatches.

These are deterministic protocol checks, not evidence of HDC or model accuracy, physical sensing correctness, or resource-meter accuracy.

## Evaluation plan

This receipt makes comparable, source-matched arms possible: DSP-only, compact `DirectMelFrameV1 / HV16`, dense `AudioHdcEncoder / ContinuousHV`, DSP + HDC late fusion, HDC-CfC temporal state, pretrained speech embeddings and audio-text embeddings.

Use source/speaker-disjoint splits, fixed source windows, pinned encoder/config/data/scorer digests and matched compute budgets. Keep ASR WER/CER separate from non-lexical acoustic event, prosody, calibration and OOD metrics. Report task quality together with latency, RTFx, RAM/VRAM, bytes per audio second and independently measured energy.

A derived representation must demonstrate incremental value over DSP and strong pretrained baselines before becoming a production dependency.
