# AV Response Window Benchmark v0

**Primary issue:** #6406  
**Related issue:** #6361 (overlapping scope; #6406 is the more complete specification)  
**AV consumer:** Symthaea #12837  
**Full-duplex floor control:** Symthaea #5169 / PR #5183  
**PCM continuity:** Symthaea #10788 / #10813 / #10894  
**Observation and replay:** Symtropy #6183 / #6201  
**Temporal eligibility:** Symtropy #6253 / PR #6254  
**Foreground/background isolation:** Symtropy #6249  
**Modality necessity:** Symtropy #6305 / PR #6307  
**Long-horizon memory:** Symtropy #6357 / PR #6359

## Research question

Does a streaming AV assistant choose the right interaction action, assign the action to the right trigger, begin inside the correct response window, stop when authorization is revoked, and provide correct content? Can it remain silent during expected-silence intervals?

This benchmark intentionally evaluates **when to act, when not to act, and when to stop**. Average response latency alone cannot capture those behaviors.

## Industry evidence and comparison targets

DIVE (October 2026) evaluates 525 full-duplex audiovisual instances across six task types. It separates participation decisions, temporal alignment and response quality. The reported best average window response rate is 53.77% for GPT-Realtime-2.1 at a reported mean latency of 0.831 seconds; its speech-proactive-intervention (SPI) window response rate is 20.0%, while MiniCPM-o 4.5 reports 24.8% on SPI. This is an important warning: sub-second mean latency does not imply good per-event response-window compliance.

OmniInteract evaluates 250 videos and 1,430 temporal response slots, including 368 continuous 1QnA slots. Its reported best IA-QTF1 is 0.368 overall and 0.052 on continuous 1QnA. StreamArena adds a distinct long-horizon online perception, retrospection, proactive-interaction and tool-use comparison. Their metrics have different denominators and protocols; do not compare these scores as if they were one leaderboard.

Official references:
- DIVE: https://arxiv.org/abs/2610.28405
- OmniInteract: https://aclanthology.org/2026.emnlp-main.490/
- StreamArena benchmark and leaderboard: https://github.com/JIA-Lab-research/StreamArena

## Normative semantics

All initial fixtures use integer replay ticks for ordering only. They are **not milliseconds** and must not be converted into wall-clock latency claims.

A response-required opportunity has one immutable trigger reference and one half-open valid response window:

- Early: response start is before the window start.
- On-time: window_start_tick <= response_start_tick < window_end_tick.
- Late: response start is at or after the exclusive window end.
- Missed: no response is assigned to the required opportunity.
- Expected silence: any assigned response is a false intervention.
- Canceled trigger: a response starting at or after cancellation, or continuing past cancellation, is stale and a hard violation.
- Duplicate delivery: repeated transport delivery with the same trigger reference does not mint a second response opportunity.
- Overlapping opportunities: each response is assigned to exactly one trigger; output intervals from one assistant cannot overlap.

Response quality is independent of participation and timing. A perfectly timed incorrect answer is not a successful response. A correct answer that arrives late is also not an on-time success.

## Fixture layout

- fixtures/agent/workload-v1.json: ten candidate-visible prompt/trigger timelines, including opaque trigger references for expected-silence observations.
- fixtures/evaluator-only/truth-v1.json: expected participation, valid windows and exact content labels.
- fixtures/agent/reference-submission-v1.json: finite authored response trace for evaluator semantics.
- schemas/: Draft 2020-12 workload, truth, submission and scorecard schemas.
- tools/av-response-window-evaluator/evaluate.py: deterministic, standard-library evaluator.
- tools/av-response-window-evaluator/test_evaluate.py: adversarial boundary, cancellation, silence and duplicate tests.

The repository is public. Evaluator truth is separated by role but is **not a blind holdout**. A real benchmark needs an evaluator-controlled media/truth service and separate candidate credentials/process isolation. Current trigger prompts are synthetic metadata only; no PCM/RGB stream is included.

## Required scorecard

Report separately:
- interaction-decision accuracy;
- expected-silence specificity and false-intervention count;
- response-required denominator;
- on-time / early / late / missed counts;
- Window Response Rate (WRR = on-time responses / all response-required opportunities);
- response content accuracy, separately overall and among on-time responses;
- response start offset P50/P95 in replay ticks;
- duplicate-response count and hard evidence/timing violations.

Expected-silence opportunities are **not** part of the WRR denominator. They have their own false-positive denominator. A candidate cannot improve WRR by answering only easy tasks if it omits other response-required events: every required opportunity remains in the denominator.

## Initial finite reference expectations

The authored reference trace covers 11 opportunities across 10 episodes: eight response-required opportunities and three expected-silence opportunities. It is expected to produce:
- 11/11 correct interaction decisions;
- 8/8 on-time responses (WRR 1.0);
- zero early, late or missed required responses;
- zero false interventions;
- 8/8 correct response contents;
- zero duplicate responses and zero hard violations.

This is a finite evaluator known-answer check, not a model score, external benchmark reproduction or SOTA claim.

## Proposed target ladder (research goals, not achieved)

| Level | Research target |
|---|---|
| R0 — semantics | 100% finite known-answer decision and half-open-window classification; zero duplicate credit |
| R1 — silence and cancellation | Zero false interventions on held-out silence controls; zero stale responses after cancellation/supersession |
| R2 — timing | Reproduce the official DIVE protocol first; target at least 60% overall WRR on a protocol-matched held-out set |
| R3 — proactive interaction | Target at least 35% SPI WRR on a sufficiently powered, held-out SPI-like cohort, without increasing false-speak rate |
| R4 — response quality | Improve content correctness at matched WRR and resource budget |
| R5 — efficiency | Improve WRR and response quality at matched p95 latency, or retain non-inferior quality at lower measured energy and unnecessary speech duration |

The R2/R3 figures are internal aspirations, not current results. For a superiority claim, reproduce the official baseline and event annotation rules first; preregister the difficult stratum; run matched hardware/model/input budgets; publish confidence intervals and false-speak rates. A Pareto improvement is more meaningful than one blended score.

## Ownership and claim limits

The evaluator is benchmark tooling only. It does not implement a voice floor controller, TTS pipeline, cancellation queue, microphone driver, streaming scheduler or a full-duplex runtime. It composes with existing Symthaea #5169 and #12837 and Symtropy #6201/#6253/#6249.

The initial suite qualifies synthetic timing semantics only. It does not measure microphone-to-speaker latency, acoustic echo/cancellation, hardware playback, human-rated response quality, real-world WRR, or SOTA superiority. Real latency requires instrumented monotonic-clock telemetry with clock qualification; replay ticks are never milliseconds.
