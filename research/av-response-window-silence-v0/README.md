# AV Response Window and Correct Silence Benchmark v0

Issue: #6492  
Turn controller: Symthaea #5169 / PR #5183  
Existing interaction benchmark: Symthaea #4773  
Foreground/background isolation: Symtropy #6249  
Decision-valued observation: Symtropy #6222 / #6225  
Modality necessity: Symtropy #6305 / PR #6307  
Long-horizon memory: Symtropy #6357 / PR #6359

## Objective

Measure whether a continuous assistant responds when needed, stays silent when appropriate, asks when the task is ambiguous, yields when interrupted, obeys a latching stop, cancels stale plans and remains responsive under background cognitive load.

This is an evaluation contract and synthetic workload. It reuses the full-duplex controller and foreground/background owners rather than adding a new scheduler, turn controller, queue, memory engine or fusion service.

## Why this is a competitive frontier

Three recent benchmarks show complementary weaknesses:

- **SPOT-Bench (NeurIPS 2026 E&D):** every prediction across the full stream is evaluated with Timeliness-F1. At 1 Hz, Ctrl-SPOT reports T-F1@5 of 20.9 overall (ABD 22.2, PNR 19.5). The benchmark reports a spam/unresponsiveness trade-off and that around half the stream can be expected silence. Timestamped QA at sampled pauses cannot measure this behavior.
- **DIVE (October 2026):** 525 AV samples across six full-duplex interaction tasks. Its leading average response-window rate is 53.77% despite a reported 0.831 s mean reaction latency; the proactive speech-intervention stratum is only 20.0% for that system. The paper separates response quality, participation decision and response timing.
- **ALIVE (2026):** reported scores include 40.3 on IPIBench, 52.8 FSI-Bench EDP and 76.0 StreamingBench. Its task model includes feedback-seeking, asking clarification and requesting user action—not just immediate answers.

Sources:
- SPOT-Bench: https://dibschat.github.io/SPOT-Bench/
- Google Research SPOT-Bench summary: https://research.google/pubs/dont-pause-every-predictions-matter-in-a-streaming-video/
- DIVE: https://arxiv.org/abs/2610.28405
- ALIVE: https://lijinzhao30.github.io/ALIVE/

These scores use different protocols and must not be pooled into one leaderboard. The benchmark-specific metrics are external reproduction targets; this synthetic scorecard is a complementary interaction-control diagnostic.

## Workload and truth planes

- Agent workload: twelve synthetic stream episodes with task prompts, event IDs, logical availability ticks, modalities and background-load profiles.
- Evaluator truth: expected action decisions, half-open response windows, silence intervals, yield/stop acknowledgement bounds, duplicate and supersession controls.
- Reference submission: finite synthetic rule-policy actions.
- Expected scorecard: known-answer aggregate metrics.

The workload is static metadata for fixture validation. A live candidate must consume observations incrementally at their availability ticks; it must not receive future stream events or evaluator windows. The truth file is in a public repository and is therefore not a blind set. Blind evaluation needs evaluator-controlled truth, hidden event timings and post-freeze probe scheduling.

Logical ticks are not wall-clock milliseconds. The reference submission is not a learned model or physical renderer run.

## Twelve initial scenario families

1. Extended dead-time with irrelevant movement and low-salience audio: remain silent.
2. Imminent collision cue: respond within a short response window.
3. Repeated/held alarm with duplicate transport delivery: respond once, not on every duplicate.
4. Ambiguous request: ask a clarifying question instead of guessing.
5. User speech begins during assistant output: yield and measure output-stop acknowledgement.
6. User backchannel during assistant speech: do not mistake it for a floor transfer.
7. Explicit stop: stop and do not resume within the same turn epoch.
8. New evidence supersedes a planned response: cancel the prior plan, do not emit stale content.
9. Slow-down command: acknowledge and de-escalate.
10. Urgent cue under saturated background memory compaction/search: preserve the response window.
11. Audio and video come from incomparable clock domains: abstain from an unqualified association.
12. Mixed long stream: remain silent during dead-time, then answer one relevant question once.

## Candidate action model

Candidate submissions report only actions that actually occurred: respond, clarify, yield, stop, deescalate, cancel, abstain or defer. Each action carries a trigger event ID, decision tick, turn epoch, optional output-start/output-stop-ack ticks, target plan ID and evidence references.

Silence is represented by the absence of a user-facing action during an expected-silence interval. This prevents a candidate from satisfying a silence metric by merely reporting an internal "silent" label while it emits audio.

## Scorecard dimensions

- Decision accuracy over all expected actions, including silence.
- Response-window hit rate, early/late/missed response counts.
- Silence-window precision and false-positive speech during dead-time.
- Unsolicited and duplicate response counts.
- Yield and stop acknowledgement success.
- Stop-latch violations and stale-response emission after supersession.
- Backchannel false-yield rate.
- Cancellation, de-escalation and abstention action counts.
- Response-window misses under background memory/search load.

These dimensions remain separate. There is no composite score that can trade correct silence for low latency or reward a controller for never responding.

## Proposed research targets (not achieved)

- 100% correct silence on deterministic dead-time controls and 100% required response-window hits on unambiguous synthetic triggers.
- Zero duplicate unsolicited interventions for repeated/held triggers.
- Zero same-epoch output after latched stop.
- Zero stale response after supersession.
- Zero false floor transfers on backchannel controls.
- Reproduce official Ctrl-SPOT T-F1@5 20.9 under its official 1 Hz protocol before targeting 23 and 25 on the same split.
- Reproduce DIVE's task-specific window response rates and response timing before targeting higher SPI/overall WRR at matched p95 latency.
- Demonstrate that #6249 background cognition does not cause foreground deadline misses under declared compute/memory pressure.
- Publish p50/p95/p99 measured response and stop-ack latency, not just mean latency.

These are research goals. Official benchmark reproduction requires licensed/available data, exact scoring protocol, model/runtime/hardware pins, repeated seeds, confidence intervals and evaluator control.

## Owner boundaries and claim limits

The runtime floor/stop/yield controller remains Symthaea #5169/#5183. Foreground/background isolation remains Symtropy #6249. EVSI/modality routing remains #6222/#6225/#6307. This issue owns synthetic scenarios and evaluation only.

A synthetic pass does not prove physical audio stopped, renderer latency, human preference, real-time determinism, universal conversation naturalness or SOTA superiority. Candidate-reported output-stop acknowledgement must be replaced with independent renderer/hardware acknowledgement for a physical timing claim.