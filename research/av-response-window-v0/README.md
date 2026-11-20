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


## Additional speech-floor baseline: Full-Duplex-Bench v3

The benchmark matrix should also include [Full-Duplex-Bench](https://github.com/DanielLin94144/Full-Duplex-Bench), especially v3. The official repository describes v3 as 100 human-recorded disfluent speech examples across 79 scenarios and 12 speakers, spanning four domains with one-to-three tool calls. Its evaluator reports tool-selection F1, argument accuracy, strict scenario pass rate and first-response / first-tool-call / task-completion latency.

This is complementary rather than directly mergeable with DIVE:
- Full-Duplex-Bench v1/v1.5 isolates pause handling, backchannels, interruption and overlapping speakers.
- v2 adds live WebRTC/WebSocket evaluation and a dynamic examiner.
- v3 adds real disfluency and multi-step tool execution.
- DIVE adds visual engagement, task-specific response windows and response-quality separation.
- OmniInteract adds streaming A/V triggers, response slots, interruption and context continuity.

**Data access and licensing matter:** the upstream README states that its CANDOR pause-handling and smooth turn-taking subsets were removed from the shared Drive in November 2026 and are available only from BetterUp under the CANDOR Corpus License. Do not use old copies or re-upload them. The synthetic and v1.5 sets remain distinct from CANDOR; check upstream per-subset terms before use. The v3 repository labels its own code/data under CC BY-NC 4.0; preserve attribution and noncommercial restrictions, and review any third-party API/model provider terms separately.

For our matched comparison, use the currently distributed, rights-qualified v3 corpus for disfluency/tool-chain behavior and the available synthetic/v1.5 overlap subset for interruption tests. Report its own official tool F1, strict pass rate and latency separately; do not combine them with AV WRR into one blended score.

A useful composed evaluation plan is therefore:
1. DIVE: visual/speech proactive intervention and response-window alignment.
2. OmniInteract: native online A/V trigger handling, proactive response and continuity.
3. Full-Duplex-Bench v3: human disfluency, turn-taking, interruption and multi-step tool execution.
4. StreamArena: hour-scale retrospection, proactive monitoring and tool use.
5. Our synthetic counterfactual, evidence-memory and response-window suites: source provenance, modality necessity, causality, cancellation, and known-answer evaluator semantics.

Primary upstream references:
- Full-Duplex-Bench main repository and latest data availability notice: https://github.com/DanielLin94144/Full-Duplex-Bench
- Full-Duplex-Bench v3 paper: https://arxiv.org/abs/2604.04847
- Full-Duplex-Bench v2 paper: https://arxiv.org/abs/2510.07838

## Renderer-confirmed output evidence (added with VOICE-FDX-001B composition)

A response interval now requires two distinct lifecycle event references:
- playback_start_event_ref: renderer-confirmed **PlaybackStarted**. This is the only event that can establish a response start.
- playback_end_event_ref: renderer-confirmed **PlaybackEnded** or **OutputStopped**.

An optional controller_permission_event_ref may be retained for audit, but it is intent only and never creates an answer or a start timestamp. An output_stopped response must cite its matching output_stop_request_event_ref; start, end and request references must be distinct and unique within the submission.

The submission also declares:
- timing_evidence_mode = synthetic_reference for the finite known-answer fixture. Its ticks validate only logical benchmark semantics.
- timing_evidence_mode = renderer_qualified for future measured runs, with a SHA-256 reference to the qualified clock relation used to map renderer timestamps into benchmark time.

The digest reference is an identifier, not cryptographic proof, renderer authentication or evidence that a physical speaker emitted sound. The adapter in Symthaea [#13014](https://github.com/Luminous-Dynamics/symthaea/issues/13014) must provide the actual lifecycle trace and the evaluator service must independently validate the clock relation. A missing PlaybackStarted or PlaybackEnded/OutputStopped event is incomplete / NOT_MEASURED, never a zero-latency response.

This preserves the boundary in the existing Symthaea #5169 controller: AssistantSpeechPermitted is not playback start; OutputStopRequested is not output stopped; natural completion is not a stop/yield acknowledgement. No timestamp conversion from controller nanoseconds to replay ticks is allowed without an explicit, independently qualified clock relation and uncertainty bounds.

### Typed renderer lifecycle ledger

The candidate submission now includes a separate renderer_lifecycle_events array. Each event has an immutable event_ref, session_epoch, turn_id, event_kind and logical tick. Scored response start/end references must resolve to:
- playback_started for the response start;
- playback_ended for natural completion; or
- output_stopped for interruption/stop completion, with a cause_event_ref pointing to a same-session, same-turn output_stop_requested event.

If controller_permission_event_ref is supplied, it must resolve to assistant_speech_permitted for the same turn and occur no later than playback start. It remains an intent record only. The evaluator never uses its tick to create a response interval.

All referenced lifecycle event IDs must be unique. Response timestamps must equal the ticks of the referenced renderer events. Output stop request, playback start and output stop acknowledgement must have a causal ordering, and the output-stop acknowledgement must identify its request. A response that only contains a permission event, a stop request without acknowledgement, or a natural end mislabeled as a stop is rejected.

The finite fixture uses timing_evidence_mode=synthetic_reference and a synthetic logical-tick relation. It validates the data contract only. Future measured submissions must use timing_evidence_mode=renderer_qualified and a SHA-256 identifier for the independently qualified clock relation. The evaluator's digest check validates identifier shape only; it does not prove the mapping or authenticate hardware. Symthaea #13014 owns the real controller/renderer trace bridge and independent clock-qualification evidence.

### CPAL 0.17.3 renderer-clock interpretation

The current Symthaea live-voice implementation uses CPAL 0.17.x in src/voice/audio_out.rs. CPAL 0.17.3's OutputCallbackInfo exposes an OutputStreamTimestamp with a callback instant and a predicted playback instant (the expected DAC delivery time). The callback currently ignores that metadata. CPAL StreamInstant is monotonic for one stream lifetime, but the origin is not guaranteed to be shared across streams. The response-window adapter must therefore preserve stream ID/generation and use same-stream arithmetic only unless an independent clock transform is qualified.

Important limits:
- CPAL playback time is a predicted device/DAC schedule timestamp, not an acoustic loopback or listener-position measurement.
- CPAL callback time is not automatically comparable with controller now_ns, std::time::Instant, another output stream, or the benchmark's replay tick.
- A callback that observes a silent buffer is not acoustic confirmation of silence at a microphone; independent loopback is needed for that stronger claim.
- Current LiveVoice stop() clears the synthesis/push flag while the ring buffer can still drain. The callback must confirm that old queued frames are no longer being emitted before an OutputStopped receipt is created.

CPAL 0.17.3 API references:
- OutputCallbackInfo: https://docs.rs/cpal/0.17.3/cpal/struct.OutputCallbackInfo.html
- OutputStreamTimestamp: https://docs.rs/cpal/0.17.3/cpal/struct.OutputStreamTimestamp.html
- StreamInstant: https://docs.rs/cpal/0.17.3/cpal/struct.StreamInstant.html
