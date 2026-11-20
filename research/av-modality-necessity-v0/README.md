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

## External protocol mapping and evaluation arms

OmniPro provides 2,700 test samples over 1,262 videos, 9,051 triggers and nine tasks. Its audio-dependency labels are required / helpful / none; 65.5% are audio-required and 18.5% audio-helpful. Use Probe mode for trigger-centred content understanding and Online mode for response timing. Map our internal strata onto those official labels only where task semantics match; do not relabel a task solely from its modality inputs. Its 84% audio-required-or-helpful coverage makes it a useful external audio-necessity complement. OmniPro's dataset card declares CC BY-NC 4.0 and evaluation-only use.

Matched evaluation arms:
- video-only model / vision-only input;
- audio-only model / raw audio input;
- transcript-mediated video (ASR transcript plus video; no raw audio);
- modular specialist audio + visual encoders with late fusion;
- strong general AV model such as Qwen3.5-Omni or an appropriately documented proprietary omni model;
- Symthaea native acoustic/HDC-CfC path;
- Symthaea hybrid path.

Use the official OmniPro Probe and Online metrics when evaluating OmniPro; use our paired counterfactual scorecard only for the synthetic local challenge. Keep model, hardware, task, input-sampling and resource budgets matched. A model's advertised input modality is not evidence that it actually used it.

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

## External benchmark data rights and use

- OmniPro's Hugging Face dataset card declares CC BY-NC 4.0 and says the benchmark is for evaluation, not model training. Its videos originate from LongVALE and COIN; those source terms remain relevant. Treat OmniPro as evaluation-only and non-commercial unless rights holders grant additional permissions. https://huggingface.co/datasets/omniproact-bench/OmniPro
- StreamArena evaluation code is Apache-2.0, but annotations are CC BY-NC 4.0 and video copyright remains with the original YouTube uploaders. Its dataset card describes video redistribution as non-commercial research reproducibility. Do not mirror or redistribute the media in this repository. https://github.com/JIA-Lab-research/StreamArena
- VSAS-Bench is an open-access paper and benchmark protocol, but the underlying media/dataset reuse license has not been verified here. Do not assume commercial or redistribution rights from paper access alone. https://openaccess.thecvf.com/content/CVPR2026F/html/Vasu_VSAS-Bench_Real-Time_Evaluation_of_Visual_Streaming_Assistant_Models_CVPRF_2026_paper.html

All external reproduction must pin dataset and evaluator revisions, record rights/consent restrictions, keep evaluation-only data out of training, and avoid redistributing copyrighted source media.

## Claim ceiling

Passing qualifies only synthetic paired-modality semantics and metadata-level split checks. It does not validate acoustic propagation, physical sensors, real-world AV model accuracy, privacy, or superiority over published benchmarks.
