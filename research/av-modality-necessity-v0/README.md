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


## Candidate submission and deterministic evaluator (added after the initial fixture tranche)

The initial implementation now has a distinct candidate-submission contract, scorecard contract and offline reference evaluator:

- \`schemas/policy-submission-v1.schema.json\`: candidate prediction, abstention, confidence, modality declaration, exact observation citations and evidence/response ticks.
- \`schemas/scorecard-v1.schema.json\`: machine-readable unweighted quality, coverage, calibration, evidence-compliance and paired-counterfactual report.
- \`tools/av-modality-necessity-evaluator/evaluate.py\`: standard-library-only deterministic scoring.
- \`tools/av-modality-necessity-evaluator/test_evaluate.py\`: adversarial evaluator tests.
- \`fixtures/agent/reference-submission-v1.json\`: finite synthetic rule-policy example.
- \`fixtures/expected/reference-scorecard-v1.json\`: expected output for that reference example.
- \`.github/workflows/av-modality-necessity-scoring.yml\`: schema, unit, adversarial and exact-scorecard checks.

Run it with:

    python3 tools/av-modality-necessity-evaluator/evaluate.py \
      --workload research/av-modality-necessity-v0/fixtures/agent/workload-v1.json \
      --truth research/av-modality-necessity-v0/fixtures/evaluator-only/modality-truth-v1.json \
      --submission research/av-modality-necessity-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/av-modality-scorecard.json

The candidate process receives the public workload and emits a submission. The evaluator consumes the submission and truth separately. **Because this repository is public, the current truth fixture is not a security boundary or a blind test set.** It demonstrates data-plane separation and deterministic scoring only. A blind benchmark must place held-out truth and media in evaluator-controlled storage or a separate restricted service, outside candidate filesystem, credentials, logs and build artifacts.

The evaluator checks episode-scoped citations, whether each cited observation was available by the declared evidence cutoff, query/response ordering, modality-to-citation consistency, transcript unavailability, required modality declarations, missing-audio abstention, common-cause non-independence and conflict/misalignment association claims.

The reference rule policy yields 13/13 correct decisions, 12/13 coverage, 1.0 selective accuracy, answered-only Brier 0.0025, 13/13 evidence-compliant decisions and zero hard violations on the **authored finite known-answer fixture**. This validates evaluator semantics; it is not a learned model score, external benchmark result, baseline or SOTA claim.

The candidate-declared modalities and citations are auditable declarations, not proof of internal model attention or causal use. Actual causal claims require real admitted PCM/RGB evidence, counterfactual modality ablations, frozen content digests, exact model/runtime/hardware revisions, repeated seeds and independent evaluator control. The current fixture contains no PCM or RGB bytes.

## Competitive evaluation ladder

External comparisons must preserve each benchmark's official protocol and use this synthetic scorecard only as a complementary diagnostic:

- **OmniPro:** run official Probe and Online modes separately, stratifying required/helpful/no-audio tasks and reporting non-speech audio separately. Do not treat “audio accepted by the API” as evidence of audio use.
- **VSAS-Bench:** reproduce the synchronous and asynchronous streaming setup; preserve response-window and temporal association metrics.
- **StreamArena:** use long-horizon perception, retrospection, proactive interaction and tool-use tasks. Report memory retention and evidence provenance alongside answer scores.
- **OmniInteract:** add sustained proactive interaction and multi-step follow-through, not just one-shot question answering.
- **DIVE:** add response-window correctness and useful intervention timing, not just mean response latency.

Matched arms should include video-only, audio-only, transcript-mediated AV, modular late fusion, a strong general AV model, Symthaea native, and Symthaea hybrid. Pin checkpoint/API revision, prompt, decoding, sample rate, channel layout, frame selection, input bytes, context limits, tools, hardware, quantization, seeds and evaluator commit. Run repeated seeds and publish confidence intervals. Keep offline and real-time results separate.

### Target ladder (research targets, not achieved results)

| Level | Qualification objective | Proposed acceptance |
|---|---|---|
| M0: evaluator integrity | Reproducible scoring and negative controls | 100% known-answer reproduction; zero truth leakage into candidate input; all adversarial tests pass |
| M1: modality contribution | Demonstrate that a necessary channel changes the decision | At least +10 percentage points paired accuracy from admitting the necessary modality, on preregistered matched held-out episodes |
| M2: temporal grounding | Correctly associate evidence only when admissible | Zero forced associations in explicit conflict/misalignment controls; no future/stale evidence use |
| M3: calibrated uncertainty | Degrade or abstain when support disappears | Report coverage-risk curves and Brier/ECE by stratum; missing necessary evidence never retains unsupported confidence |
| M4: continuous interaction | Useful response-window timing, sustained monitoring and interruption handling | Match official benchmark metrics first; then improve response-window correctness at matched p95 latency and compute budget |
| M5: robust deployment | Maintain behavior under domain and sensor shift | Report source-held-out, environment-held-out, sensor dropout/drift and common-cause stress results with confidence intervals |

Do not collapse these levels into one weighted leaderboard score. Hard evidence and temporal violations must remain visible gates. The evaluator currently computes task, coverage, selective accuracy, Brier, paired discrimination, evidence compliance and deadline misses. Submission fields reserve latency, bytes, frames, tokens and energy; their aggregation and measured-run comparison remain future work rather than inferred from simulation ticks.

For external SOTA claims, first reproduce the selected benchmark's official baseline and evaluation environment. A reasonable next claim is a non-inferiority result at materially lower measured resource cost, or a statistically significant improvement on a preregistered failure stratum at matched resources—not a broad claim based on the 13 synthetic episodes.


### Safe abstention semantics

The offline evaluator treats an explicit abstention as a correct safe decision in the conflict and temporal-misalignment strata, as well as in the missing-audio stratum. Answer accuracy remains false for an abstention, coverage decreases, and answered-only calibration is reported separately. This avoids forcing a model to invent an association merely to maximize answer accuracy. An abstention does not excuse a declared false association or other evidence-contract violation.


## Published benchmark anchors (checked 20 November 2026)

These are external protocol-specific reference points, not directly comparable scores and not results from this project.

| Benchmark | Published scale / standout result | Implication for this programme |
|---|---|---|
| OmniPro (May 2026) | 2,700 samples, 1,262 videos, 9,051 triggers, nine tasks; 84% require or benefit from audio (65.5% required, 18.5% helpful) | Preserve required/helpful/none strata, non-speech cases and Probe vs Online protocol |
| VSAS-Bench (CVPR 2026) | 18,000+ annotations; Qwen3-VL-4B adapted for streaming exceeds Dispider by 3 percentage points on the asynchronous protocol | Include strong adapted offline VLM baselines; streaming adaptation itself is a serious comparator |
| OmniInteract (EMNLP 2026) | 250 videos / 1,430 response slots; best overall IA-QTF1 0.368 and best continuous 1QnA IA-QTF1 0.052 | Score continuous monitoring, nested work and response timing independently from one-shot accuracy |
| DIVE (October 2026) | 525 AV samples across six interaction tasks; GPT-Realtime-2.1 leads average window response rate at 53.77%, while average latency is 0.831 s | Optimize *in-window correctness*, not merely mean latency; speech-driven proactive intervention is a hard stratum |
| StreamArena (leaderboard update 17 September 2026) | StreamMind v2 Mixed scores 55.9 / 49.1 / 70.2 / 23.3 on RTP / HR / Tool / Pro; offline Gemini 3.5 Flash scores 51.3 / 51.4 / 70.8 on RTP / HR / Tool | Evaluate long-horizon memory, retrieval, proactive behavior and tools; compare online-vs-offline under identical task splits |

Primary benchmark links:
- OmniPro paper: https://arxiv.org/abs/2605.18577 ; dataset card: https://huggingface.co/datasets/omniproact-bench/OmniPro
- VSAS-Bench CVPR paper: https://openaccess.thecvf.com/content/CVPR2026F/html/Vasu_VSAS-Bench_Real-Time_Evaluation_of_Visual_Streaming_Assistant_Models_CVPRF_2026_paper.html
- OmniInteract EMNLP paper: https://aclanthology.org/2026.emnlp-main.490/
- DIVE paper: https://arxiv.org/abs/2610.28405
- StreamArena leaderboard and evaluation toolkit: https://github.com/JIA-Lab-research/StreamArena

### Practical competitive strategy

Do not chase a single model-wide score first. Build a Pareto frontier with (1) task correctness, (2) valid response-window rate, (3) evidence integrity and modality necessity, (4) long-horizon recall, (5) calibration/abstention, (6) p50/p95/p99 latency, (7) compute/energy and (8) per-task failure rates.

The most promising opportunities suggested by the external results are:
1. **Timing-aware cognition:** preserve a low-latency foreground path and move compression/retrieval/model improvement to isolated background workers; optimize response-window hit rate rather than average latency alone.
2. **Causal modality routing:** use counterfactual evidence necessity and expected decision value to select audio/video processing, instead of always paying for full fusion.
3. **Persistent evidence memory:** retain source-linked multimodal events and replayable evidence anchors; measure long-horizon retrieval under compression rather than text-summary recall alone.
4. **No-response competence:** score correct silence, abstention, interruption handling and cancellation. A model that responds to every trigger can look active while being less useful.
5. **Composed specialists:** compare native omni models with audio specialists, vision specialists, a scheduler/router, persistent memory and independent evidence audit under the same end-to-end budget.

StreamArena's posted leaderboard illustrates why composition is worth testing: its Mixed online system uses different backbones for frontend/router/search/recall/reviewer, memory writing and monitoring. This is an external example of a composed system, not a result reproduced here.

All scores above retain their source-specific definitions, splits and hardware/API assumptions. Reproduce each official evaluation independently before claiming a win. The 13 synthetic episodes and offline scorecard in this PR are a contract/evaluator development set, not a comparable benchmark submission.
