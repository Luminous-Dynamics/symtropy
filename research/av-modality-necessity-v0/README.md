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


## Matched modality-ablation tournament (v1)

A single policy submission can report which modalities it claims to have used, but that does not demonstrate the incremental value of a modality. The benchmark now includes a matched three-arm tournament:

- **V-only:** candidate receives only the public visual observations.
- **A-only:** candidate receives only admitted raw PCM observations.
- **A+V:** candidate receives both raw modalities.

Each run is wrapped by \`schemas/arm-run-v1.schema.json\`. The comparator requires common comparison ID, policy, model/runtime revision, hardware identity, random seed, workload reference, protocol, shared policy configuration and resource caps. The arm's allowed modality set is fixed, and submissions that cite/use an unavailable modality or use a transcript shortcut are rejected.

Run the finite reference tournament:

    python3 tools/av-modality-necessity-evaluator/compare_arms.py \
      --workload research/av-modality-necessity-v0/fixtures/agent/workload-v1.json \
      --truth research/av-modality-necessity-v0/fixtures/evaluator-only/modality-truth-v1.json \
      --video-only research/av-modality-necessity-v0/fixtures/agent/arm-video-only-v1.json \
      --audio-only research/av-modality-necessity-v0/fixtures/agent/arm-audio-only-v1.json \
      --audio-video research/av-modality-necessity-v0/fixtures/agent/arm-audio-video-v1.json \
      --output /tmp/av-modality-ablation-tournament.json

The result compares A+V against V-only and A-only on the exact same episode IDs. It reports per-arm answer accuracy, coverage, selective accuracy, safe-decision accuracy and evidence violations; paired answer wins/losses/ties; net answer-accuracy gain; and per-necessity-stratum results. It includes an exact two-sided McNemar/binomial p-value for descriptive paired discordance. With only 13 synthetic cases that p-value is an evaluator diagnostic, not statistical evidence of model superiority. It must not be reported as a meaningful significance claim until a sufficiently powered held-out corpus and preregistered analysis are available.

**Abstention semantics:** safe-decision credit depends on the experiment's actual admitted input mask and observation availability at the declared evidence cutoff—not on the candidate's self-reported modality use. If a necessary modality was not admitted, was absent, or had not arrived by the cutoff, an abstention may count as a correct *decision*. If the necessary modality was available but the candidate simply chose not to use it, abstention is not credited as a correct decision. A safe abstention is still unanswered: it increases neither answer accuracy nor coverage. Conflict and temporal-association tasks may also be safely abstained. Common-cause, conflict and temporal-association tasks require both raw modalities for a substantive answer; the evaluator rejects answering these from only one channel.

The three committed arm fixtures use an authored synthetic reference rule policy, no ML model, no physical hardware and no raw sensor media. Their measured/declared cost fields are not used to claim efficiency. The common budget envelope establishes protocol matching only; actual cost comparisons require per-episode measured input bytes, frame counts, token use, latency and energy, plus repeated seeds and hardware-level metering.

The scoring workflow now runs the single-submission adversarial tests, matched-arm tests, both scorecards and all JSON Schema checks. Exact-head Actions results remain the authority for CI qualification; source-authored fixtures do not substitute for runner results.

## Research interpretation and falsifiable hypotheses

This tournament opens a more useful question than "is an AV model better than a V model?":

1. **Marginal information value:** on which task strata does A+V outperform V-only and A-only, and on which does it add no useful information?
2. **Selective compute:** can an active router achieve the same answer accuracy and response-window rate with fewer input bytes, frames and joules by invoking the second modality only when its expected decision value is positive?
3. **Temporal association:** does preserving measurement intervals and clock uncertainty reduce false event matching compared with timestamp concatenation or nearest-neighbour fusion?
4. **Dependence-aware corroboration:** can source-dependency groups prevent correlated audio/video cues from inflating confidence?
5. **Sustained interaction:** does the system maintain useful monitoring and make correct non-response decisions over long streams, rather than optimize one-shot QA?

The proposed SOTA route is to establish strong matched baselines on each official benchmark, then show one of two narrow results:
- a statistically supported improvement on a preregistered failure stratum at matched hardware/input/latency/energy budgets; or
- non-inferior task quality and response-window correctness at materially lower measured resource cost.

A broad industry-leadership claim requires multiple independent datasets, source-held-out and environment-held-out tests, repeated seeds, confidence intervals, and independent evaluation. The synthetic scorecard and 13-case tournament are engineering scaffolding for that research, not evidence that the target has been achieved.


## DIVE-specific gap analysis (20 November 2026)

The DIVE paper provides a particularly useful design target because it explicitly separates interaction decisions, temporal alignment and response quality. Its 525 evaluation instances span six task types and 413 base scenarios. GPT-Realtime-2.1 reports a 53.77% mean window response rate and 0.831 s mean latency; however, its speech-proactive-intervention (SPI) window response rate is only 20.0%. MiniCPM-o 4.5 reaches 24.8% on SPI. The benchmark also reports 100% VTI for GPT-Realtime-2.1 and Qwen3.5-Omni-Plus-Realtime, showing how an overall average can hide weak task-specific performance.

DIVE's authors explicitly list several unmeasured areas: agentic tool use, longer-duration interaction with persistent multimodal memory, long-range dependencies and controlled within-model ablations isolating visual input. Those are unusually direct opportunities for this programme. The AV modality tournament addresses controlled A/V ablation; a separate long-horizon extension should address persistent evidence retention, correction and retrieval. We must not claim to outperform DIVE without running its official tasks and evaluation code.

For exact DIVE reproduction, preserve the three distinct outputs:
- **Interaction decision:** respond when required and remain silent when the task expects silence.
- **Temporal alignment:** classify every required event as missed, early, on-time or late; on-time uses the benchmark's half-open response window.
- **Response quality:** content correctness of assigned responses, separately from timing and decision behavior.

Do not reduce DIVE to "mean response latency". Its window-response denominator is the number of response-required events, and content correctness is reported separately. Our own future interaction scorecard should similarly publish (a) expected-silence false-positive rate, (b) missed-required-event rate, (c) early/on-time/late response fractions, (d) content accuracy, (e) interruption/revision correctness, and (f) p50/p95/p99 time-to-first-safe-response. Silence and cancellation are first-class outcomes, not missing data.

The DIVE cohort's open-model latency/quality gap also suggests a focused path: use existing low-latency native acoustic/visual state for trigger detection and evidence updates, while deferring expensive specialists to bounded background work; compare that hybrid against native omni models and best-practical modular systems at the same input and hardware budget. This is a hypothesis to test, not a result.


### Resource accounting for real matched runs

Each arm envelope now declares \`measurement_status\`:

- \`synthetic_reference\`: the evaluator rejects any latency/energy/input-cost claim and returns resource metrics as null.
- \`measured\`: every episode must provide audio bytes, video frames, input tokens, output tokens, end-to-end latency in milliseconds and energy in joules. The comparator checks each value against the identical per-episode budget caps and requires zero bytes/frames for a modality not admitted to that arm.

Measured-run output aggregates total audio bytes, frames, input/output tokens and energy, plus nearest-rank p50/p95/p99 response latency. It does not infer these quantities from replay ticks. The arm fixtures are explicitly marked \`synthetic_reference\`, and their resource fields remain unavailable rather than fabricated.

The comparator verifies the workload's exact Git blob object ID against the input file bytes, and checks that each nested submission's model/runtime revision matches the outer run envelope. This makes the matched protocol's dataset and implementation identity auditable before any quality-versus-cost comparison.

**Competitive metric:** publish a Pareto frontier, not a single scalar winner: answer accuracy, safe-decision accuracy, coverage, selective accuracy, hard evidence violations, paired A+V gain, response-window correctness, p95 latency, total energy and input bytes. A candidate that saves energy by missing events or abstaining on everything must not appear superior.


### Evaluator incentive audit

The candidate's declared \`used_modalities\` is no longer treated as the availability mask. The comparator supplies each arm's actual permitted inputs to the evaluator, which independently derives whether the required modality had arrived by the prediction's declared evidence cutoff. This prevents an A+V policy from receiving safe-abstention credit after choosing to ignore an audio observation it was actually given. The regression suite includes both the genuinely missing-input case and the deliberate-underuse case.


## Expanded competitive benchmark matrix (checked 21 November 2026)

The field now has several stronger and more diverse baselines than generic video-QA. We should compare within each benchmark's own protocol and then use our evidence scorecards as complementary diagnostics.

| Work | Published scale / result | What it contributes to our evaluation |
|---|---|---|
| **SPOT-Bench (NeurIPS 2026 E&D)** | At 1 Hz, Ctrl-SPOT reports T-F1@5 20.9 overall (22.2 ABD, 19.5 PNR); the benchmark reports that roughly half the stream is expected silence | Full-stream detection, timeliness and false-positive/silence control |
| **ALIVE (2026)** | Its page reports 68.9 OVO-Bench, 76.0 StreamingBench, 51.6 ProactiveVideoQA PAUC, 40.3 IPIBench and 52.8 FSI-Bench EDP | Strong streaming interaction baseline: task coordination, proactive monitoring and feedback-seeking |
| **Realtime-Venus (September 2026 report)** | Realtime-Venus-Omni reports 70.2 StreamingBench; the separate audio model reports 78.0 MMAU and 97% C_RESUME on Full-Duplex-Bench v1.5 | A two-loop architecture with live interaction, background delegation and retrieved long-video frames |
| **4DMemory (October 2026)** | LongVideo: about 900 hours and 5,000+ reviewed questions; its harness combines four skills and 22+ specialist models, with >50% improvement over the corresponding standalone model results reported by authors | Large-scale long-horizon spatiotemporal memory; our challenge must add source lineage, corrections, dependence, clock admissibility and revocation |
| **EgoMemReason (May 2026)** | 500 questions over week-long egocentric streams; average evidence backtracking 25.9 hours; best reported overall accuracy 39.6% | A long-horizon memory reference for entity, event and behavior memory across days |
| **DIVE (October 2026)** | 525 AV samples / six task types; leading average response-window rate 53.77% despite 0.831 s mean latency | Response-window correctness, proactive intervention timing and full-duplex participation |
| **OmniInteract (EMNLP 2026)** | 250 videos / 1,430 response slots; best IA-QTF1 0.368 overall and 0.052 on continuous 1QnA | Trigger detection, timing, interruption handling, nested task completion and continuity |

Primary sources:
- SPOT-Bench: https://dibschat.github.io/SPOT-Bench/
- ALIVE: https://lijinzhao30.github.io/ALIVE/
- Realtime-Venus: https://realtime-venus.github.io/ ; technical report https://arxiv.org/abs/2609.13814
- 4DMemory: https://arxiv.org/abs/2610.32332
- EgoMemReason: https://arxiv.org/abs/2605.09874
- DIVE: https://arxiv.org/abs/2610.28405
- OmniInteract: https://aclanthology.org/2026.emnlp-main.490/

### How to avoid losing to these systems

1. **Do not compare a 9-episode metadata fixture to a 900-hour dataset.** Reproduce official splits and scoring first. Use LongVideo/4DMemory and EgoMemReason as external memory workloads where licensing and artifact access permit; use our own hidden provenance/correction/clock/revocation probes as an orthogonal qualification layer.
2. **Do not optimize only for response latency.** SPOT-Bench reports spam/unresponsiveness trade-offs and dead-time; DIVE reports response-window failures despite sub-second mean latency. Optimize timeliness-F1/window hit rate and expected-silence false positives together.
3. **Do not assume a memory harness wins because it stores more.** 4DMemory demonstrates that a specialist harness can materially improve long-horizon results, and Realtime-Venus shows frame selection by relevance and novelty with an asynchronous task loop. Our differentiator must be source-backed, causality-preserving memory that remains correct under correction, dependency, clock reset and revocation—not just recall score.
4. **Do not use one giant-model baseline only.** Compare single omni, streaming-adapted VLM, specialist composition and evidence-ledger memory. Realtime-Venus and ALIVE show that interaction policy, delegation, feedback-seeking and learned silence are part of the capability.
5. **Report Pareto frontiers and failure strata.** Accuracy, event retrieval, provenance, abstention/coverage, response-window rate, p95/p99 latency, tokens, memory bytes, energy and hard violations should remain separate dimensions.

### Proposed stretch goals after official reproduction

- SPOT: first reproduce Ctrl-SPOT's published 20.9 T-F1@5 under the same 1 Hz task protocol; then target 23 and 25 on the official hidden split without increasing dead-time false positives. These are aspirational.
- ALIVE: reproduce the five published comparison metrics; target matched or better FSI-Bench EDP and proactive metrics while preserving evidence citation, cancellation and rights boundaries.
- DIVE / OmniInteract: reproduce response-window, interruption, nested-chain and continuous 1QnA metrics separately; improve p95 response windows and 1QnA at matched compute, not just aggregate QA.
- 4DMemory / EgoMemReason: reproduce the official long-horizon test and report quality by evidence span, memory budget, retrieval depth and modality. Target non-inferior event recall with materially lower memory cost, then separately demonstrate correction/revocation/provenance correctness.
- Every target requires exact model/runtime revision, repeated seeds, hardware disclosure, confidence intervals and evaluator-controlled held-out truth. The current synthetic reference arms are not eligible for external leaderboard claims.


## Newly relevant streaming-evaluation work (checked 21 November 2026)

Three recent benchmarks sharpen the comparison protocol:

- **SPOT-Bench (NeurIPS 2026 E&D):** it scores every prediction across the full stream with Timeliness-F1. Its posted 1Hz T-F1@5 baseline Ctrl-SPOT is 20.9 overall (22.2 ABD / 19.5 PNR). Its analysis notes roughly half the stream is dead-time where no response is expected; always-on output spam and over-conservative silence are distinct failure modes. https://dibschat.github.io/SPOT-Bench/
- **TRACE (25 September 2026):** 1,240 records from 517 videos; the central contribution is the causal Core–Adapter protocol plus explicit temporal validity, history processing, response timing, false alarms, misses, workload and completion. Similar QA accuracy can mask very different execution behavior. https://arxiv.org/abs/2609.30670
- **OmniInteract (EMNLP 2026):** the benchmark reports 1,430 response slots across 250 videos, including 368 continuous 1QnA slots. The best reported continuous 1QnA IA-QTF1 is 0.052, and overall IA-QTF1 peaks at 0.368. https://aclanthology.org/2026.emnlp-main.490/

**Implication:** do not report just response correctness and latency. Include every candidate output (including silence), exact evidence-validity window, false-alarm intervals, missed trigger windows, interrupted/nested task resumption, history workload and response timing. Use SPOT's published protocol as a proactive baseline and TRACE as a causal-execution audit. These are separate external benchmarks; the synthetic AV evaluator is complementary, not a replacement.
