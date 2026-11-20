# AV Modality Necessity Benchmark v0

**Issue:** #6305  
**AV consumer:** Symthaea #12837  
**Replay/transport:** Symtropy #6201  
**Temporal eligibility:** Symtropy #6253  
**Camera producer adapter:** Symtropy #6301 / PR #6302  
**External anchors:** OmniPro, VSAS-Bench and StreamArena.

## Research question

Does adding audio to video (or video to audio) improve task-relevant understanding on matched episodes, and can a system recognize when modalities are redundant, conflicting, stale, missing or temporally misaligned?

This is metadata-only synthetic benchmark input. It does not synthesize a physically propagated sound field, render a camera, or qualify a microphone model.

## Paradigm

Measure modality contribution with paired counterfactuals, not aggregate AV score alone:
- same video, different audio;
- same audio, different video;
- both required for the full answer;
- conflicting sources;
- same event seen/heard through dependent views;
- non-speech audio;
- temporal misalignment and missing audio;
- randomized nuisance/background cues.

The evaluator records whether A-only, V-only or A+V is sufficient. A fused model must not gain confidence merely by counting correlated views twice.

## Fixture layout

- fixtures/agent/workload-v1.json: only public tasks and admitted synthetic observation metadata.
- fixtures/evaluator-only/modality-truth-v1.json: hidden event truth, modality necessity and source-dependency labels.
- fixtures/expected/known-answer-v1.json: finite expected sufficiency classes.

The metadata contains synthetic identifiers and placeholder content commitments for pair-identity testing. It does not contain raw PCM/RGB evidence or physically qualified sensor claims. The artifact/transport integration remains owned by #6201/#6301.

## Evaluation arms

Video-only; audio-only; modular late fusion; transcript-mediated video; strong general AV model; Symthaea native; Symthaea hybrid. Keep model, hardware, task and resource budgets matched. A model's advertised input modality is not evidence that it actually used it.

## Core invariants

- Audio-necessary pair: video digest must be identical and audio digest must differ.
- Video-necessary pair: audio digest must be identical and video digest must differ.
- Common-cause audio/video views share a dependency group; they are not independent confirmations.
- A missing/dropped audio stream has no payload.
- A misaligned pair must not be associated just because event labels or file order match.
- Transcript-only evidence cannot satisfy a non-speech audio task.
- Hidden modality necessity and answer labels stay evaluator-only.

## Metrics and proposed targets

Report accuracy per necessity stratum, paired gain from adding each modality, event-source association, temporal localization, conflict handling, common-cause double-counting, calibrated confidence changes, abstention, non-speech audio accuracy, p50/p95/p99 latency, frames/audio bytes/tokens and measured cost.

Proposed gates (not achieved):
- 100% finite known-answer classification;
- zero evaluator/future leakage;
- zero independent corroboration claims from common-cause duplicate views;
- zero forced associations in conflict or temporal-misalignment controls;
- >=80% accuracy per modality-necessary stratum;
- >=10 percentage-point paired improvement from adding the necessary modality;
- missing necessary modality causes abstention or explicit loss of support;
- 100% deterministic replay.

## Claim ceiling

Passing qualifies only synthetic paired-modality semantics and metadata-level split checks. It does not validate acoustic propagation, physical sensors, real-world AV model accuracy, privacy, or superiority over published benchmarks.
