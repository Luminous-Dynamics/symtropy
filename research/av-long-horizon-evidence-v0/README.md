# AV Long-Horizon Evidence Memory Benchmark v0

**Issue:** #6357  
**AV consumer:** Symthaea #12837  
**Memory owner:** Symthaea #10644  
**As-known evidence:** Symthaea #9075 / #9177  
**Replay:** Symtropy #6183 / #6201  
**Foreground/background isolation:** Symtropy #6249  
**Decision-valued observation:** Symtropy #6222 / #6225  
**Modality necessity:** Symtropy #6305 / PR #6307

## Objective

Evaluate whether a continuous audiovisual system can retrieve events after long delays, preserve exact source provenance, distinguish similar events, apply corrections and revocations, and resist false memories after compression and distraction.

This is a first data-plane tranche: strict schema, synthetic candidate-visible workload, evaluator-only truth, expected known-answer labels and exact-head fixture integrity. It does not implement a memory engine or claim model performance.

## Why this matters

DIVE (October 2026) reports 525 full-duplex AV evaluation instances across six tasks. Its authors explicitly identify persistent multimodal memory, long-range dependencies and longer-duration interaction as areas outside its current scope. OmniInteract includes 368 continuous 1QnA slots across 250 videos and reports weak performance on that task class. StreamArena supplies a separate long-horizon online perception, retrospection, proactive and tool-use comparison.

Our contribution should test more than whether a summary contains a noun. It should test whether a system can recover the correct event, the actual observation supporting it, its measurement time and clock generation, later corrections, dependency lineage and current eligibility.

References:
- DIVE: https://arxiv.org/abs/2610.28405
- OmniInteract: https://aclanthology.org/2026.emnlp-main.490/
- StreamArena: https://github.com/JIA-Lab-research/StreamArena

## Data planes

- fixtures/agent/workload-v1.json: synthetic observation stream and future query. A real consumer must receive events incrementally at their declared availability tick; it must not mount the entire future timeline.
- fixtures/evaluator-only/truth-v1.json: hidden answer, required source references, supersession/revocation truth and dependence labels.
- fixtures/expected/known-answer-v1.json: finite evaluator-known answers for schema/integrity development.

This repository is public. The evaluator-only fixture is logically separate but **not secret or a blind holdout**. A genuine blind test must use evaluator-controlled media/truth storage, separate credentials and a process/container boundary with no candidate access to truth, logs or build artifacts.

Every digest is a placeholder commitment for metadata contract tests. No PCM or RGB bytes, physical capture or calibrated sensors are included.

## Initial nine scenarios

1. Delayed recall after ten minutes.
2. Repeated-event disambiguation with a duplicate transport delivery that reuses the original observation identity.
3. A correction that supersedes an earlier observation while preserving both revisions.
4. Audio/video views sharing one source-dependency group.
5. A rare early event amid later nuisance observations.
6. An absent event that must not be hallucinated.
7. Independent clock domains and reset generations that must remain incomparable without a qualified transform.
8. Revocation of an earlier observation, making that evidence ineligible for later retrieval.
9. Retrieval of a rare early event under a constrained memory budget and many intervening distractors.

The timeline contains availability ticks, not wall-clock latency measurements. A correction or revocation becomes effective only at its declared event availability. A replayed observation does not become a new source event. A summary cannot override the original evidence references or clock identity.

## Memory conditions to compare

The workload declares four comparison lanes:
- recent-window only;
- summary-only;
- episodic retrieval using existing Symthaea ownership;
- source-linked evidence ledger.

These are evaluation conditions, not implementations added by this PR. Compare identical streams, query ticks, model/runtime revisions, hardware, seeds and memory budgets. Keep offline hindsight analysis separate from online causal ingestion.

## Required future scorecard

Report separately:
- exact event-identity accuracy by horizon;
- answer accuracy and abstention/coverage;
- observation-citation precision and provenance completeness;
- temporal localization and event-order accuracy;
- duplicate-delivery counting errors;
- correction/supersession correctness at each as-of time;
- false-memory rate for absent events and distractor substitutions;
- common-cause double-count rate;
- cross-clock claims made without a qualified transform;
- revoked/expired evidence retrieval rate;
- retention versus memory bytes/tokens and retrieval latency;
- p50/p95/p99 latency and energy only from measured runs.

An answer without correct source lineage is not a fully qualified evidence retrieval. A no-answer decision when evidence is missing is distinct from a correct event answer.

## Proposed research targets (not achieved)

- 100% exact replay of frozen synthetic streams.
- 100% correction/supersession and revocation known-answer compliance.
- Zero duplicate-event inflation from a replayed transport delivery.
- Zero retrieval of revoked/expired evidence as admissible support.
- Zero false positive for absent-event and explicit distractor controls.
- At least 95% exact event identity on finite synthetic known-answer fixtures.
- At least 95% source-citation correctness, including interval and clock generation.
- Publish quality/coverage/latency degradation curves at 1 / 5 / 15 / 60-minute horizons.
- At matched answer quality, target materially lower retained memory bytes than a naive rolling-context baseline without loss of event/citation recall; quantify the Pareto frontier rather than claim from synthetic placeholders.

Targets must be calibrated against a powered held-out corpus and independent labels. The synthetic nine-case suite is contract scaffolding, not a SOTA benchmark submission.

## Ownership and limits

This tranche reuses #10644, #9075/#9177, #6249 and #6201. It does not create a second memory index, event graph, clock service, scheduler, compression algorithm, sensor pipeline or retrieval system. Passing fixture integrity validates metadata semantics only; it does not validate real model memory, physical AV evidence, user deletion propagation, privacy, latency or SOTA superiority.

## Next implementation

1. Add a candidate-submission contract for answers, abstention, cited observation refs, as-of tick, and memory condition.
2. Add a deterministic evaluator for answer correctness, source citation, event ordering, supersession, duplicate delivery, false memory and revocation.
3. Add a matched memory-condition tournament under exact memory/token budgets.
4. Connect the existing Symthaea memory owner and causal AV consumer without duplicating their runtime systems.
5. Move truth/media to a restricted evaluation service for blind, rights-qualified tests.

## Candidate submission and deterministic evaluator (v1)

The first evaluator tranche is now implemented:

- schemas/submission-v1.schema.json: candidate policy answer/abstention, confidence, memory condition, cited and current observation references, independence/cross-clock claims and evidence/response ticks.
- schemas/scorecard-v1.schema.json: answer/decision accuracy, coverage, selective accuracy, source-citation completeness/precision, hard violations and deadline misses.
- tools/av-long-horizon-evidence-evaluator/evaluate.py: standard-library-only deterministic scorer.
- tools/av-long-horizon-evidence-evaluator/test_evaluate.py: adversarial cases for duplicate delivery, source identity, correction/supersession, common-cause dependence, absent-event false memory, clock incomparability, revocation, citations and timing.
- fixtures/agent/reference-submission-v1.json: authored finite reference evidence-ledger policy.
- fixtures/expected/reference-scorecard-summary-v1.json: frozen reference totals.
- The exact-head workflow now validates both schemas, runs the evaluator test suite, scores the reference submission and compares the expected totals.

Run the reference scorer:

    python3 tools/av-long-horizon-evidence-evaluator/evaluate.py \
      --workload research/av-long-horizon-evidence-v0/fixtures/agent/workload-v1.json \
      --truth research/av-long-horizon-evidence-v0/fixtures/evaluator-only/truth-v1.json \
      --submission research/av-long-horizon-evidence-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/av-long-horizon-reference-scorecard.json

The evaluator requires the submission's evidence cutoff to equal the query tick. This prevents a policy from moving the evidence horizon backwards to make required observations appear unavailable. It verifies citations against the actual stream, requires required source references for answers, tracks the original observation identity across duplicate delivery, rejects revoked evidence after revocation, rejects superseded observations as the current state, and detects false independent-corroboration and cross-clock-order claims.

The finite reference submission produces eight correct event answers and one correct absent-event abstention: 8/9 answer accuracy, 8/9 coverage, 9/9 safe-decision accuracy, 9/9 complete source-citation sets, zero hard violations and zero deadline misses. The mean citation precision is 8.5/9 because the correction case intentionally cites both the superseded historical observation and the current replacement while marking only the replacement as current. These values are synthetic evaluator known answers, not learned-model scores.

### Interpretation boundaries

- Historical evidence may remain cited to explain a correction, but the superseded source cannot be claimed as current.
- Revoked evidence cannot remain eligible support after the revocation event becomes available.
- Re-delivery of the same observation reference is not a new event; distinct-event counts operate on unique source observation identities, not transport deliveries.
- An absent-event query is a success only when the policy abstains without inventing a supporting observation.
- Within-clock sequence reasoning does not license cross-clock ordering. Different domains/generations remain incomparable until an existing owner supplies a qualified transform.
- The current scorecard does not yet compare the four memory conditions or measure memory bytes, retrieval latency or energy. Those will require a separate matched run envelope and measured telemetry.

### Four-condition memory tournament known-answer correction

The exact-head Actions run on 20 November 2026 completed workload/schema checks, all 16 long-horizon evaluator tests, all 7 matched-memory tournament tests, both score-generation steps and scorecard schema validation. Its final comparison exposed that the frozen summary-only expected row understated the stricter evaluator's result. The expected fixture has now been corrected to the computed known answer:

- summary-only answer accuracy: 2/9 (0.2222);
- safe-decision accuracy: 3/9 (0.3333);
- coverage: 8/9 (0.8889);
- citation precision/recall: 0.8 / 0.8;
- provenance completeness: 4/9 (0.4444);
- evidence compliance: 4/9 (0.4444);
- false-memory count: 6;
- hard violations: 6.

The six violation count is not the same as the number of non-compliant episodes: one revocation episode carries two distinct failures—reuse/residue of revoked evidence and failure to cite the still-required current public observation. Other failures are the superseded state used as current, the common-cause views claimed independent, unsupported cross-clock ordering, and substitution of a later nuisance event for the rare target event. The corrected expected summary changes only the summary-only evidence-compliance rate and hard-violation count; task/citation outcomes remain unchanged. This is an evaluator known-answer correction, not evidence that any learned memory system has achieved these scores.


A second exact-head scoring correction records the current episodic-retrieval reference as **3 hard-violation codes across 2 non-compliant episodes**, not 2 codes. The correction case contributes one code for marking the superseded observation as current. The revocation case contributes two distinct codes: post-revocation evidence/derived-memory reuse, and omission of the still-required eligible public-action observation. The episode count and violation-code count are deliberately different metrics. This correction aligns the expected fixture with the evaluator's fail-closed semantics; it does not alter the underlying policy fixture or convert its failures into passes.
