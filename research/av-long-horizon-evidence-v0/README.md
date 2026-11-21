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

## Initial scenarios and temporal checkpoint extensions

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

Targets must be calibrated against a powered held-out corpus and independent labels. The synthetic 11-case suite is contract scaffolding, not a SOTA benchmark submission.

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

The 12-episode reference submission produces eleven correct event answers and one correct absent-event abstention: 11/12 answer accuracy, 11/12 coverage, 12/12 safe-decision accuracy, 12/12 complete source-citation sets, zero hard violations and zero deadline misses. The 15-span chain requires all fifteen source references in their exact temporal order. These values are synthetic evaluator known answers, not learned-model scores.

### Interpretation boundaries

- Historical evidence may remain cited to explain a correction, but the superseded source cannot be claimed as current.
- Revoked evidence cannot remain eligible support after the revocation event becomes available.
- Re-delivery of the same observation reference is not a new event; distinct-event counts operate on unique source observation identities, not transport deliveries.
- An absent-event query is a success only when the policy abstains without inventing a supporting observation.
- Within-clock sequence reasoning does not license cross-clock ordering. Different domains/generations remain incomparable until an existing owner supplies a qualified transform.
- The current scorecard does not yet compare the four memory conditions or measure memory bytes, retrieval latency or energy. Those will require a separate matched run envelope and measured telemetry.

### Four-condition memory tournament known-answer correction

The original 20 November exact-head Actions run validated the initial long-horizon evaluator and four-condition tournament. On 21 November the evaluator was hardened for as-known checkpoints, derived-memory revocation lineage and exact ordered evidence chains. The current 12-episode fixture and four matched arms are the active known-answer reference; their exact-head Actions run is the qualification authority:

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


## Expanded long-horizon benchmark comparison (21 November 2026)

The closest external comparison is no longer a short-context video-QA benchmark:

| Benchmark | Scale / result | Relevance and limitation |
|---|---|---|
| **4DMemory / LongVideo (October 2026)** | About 900 video hours and 5,000+ manually reviewed questions; authors report a harness with four skills and 22+ specialist models that improves tested foundation-model results by over 50% | The most important scale reference for long-horizon visual/spatiotemporal memory; does not by itself evaluate our source revocation, clock admissibility or derived-memory lineage |
| **EgoMemReason (May 2026)** | 500 questions over week-long egocentric video; about 25.9 hours average evidence backtracking; best reported overall accuracy 39.6% | Tests entity, event and behavior memory across days; useful external memory stress test, but visual-only and a different task protocol |
| **StreamArena (2026)** | Long-horizon online perception, retrospection, proactive interaction and tool-use evaluations | A strong online agent comparison; keep its official task splits and scoring |
| **OmniInteract (EMNLP 2026)** | 250 videos / 1,430 response slots; best reported continuous 1QnA IA-QTF1 0.052 | Measures sustained monitoring and interaction timing rather than only delayed factual recall |
| **DIVE (October 2026)** | 525 AV samples / six task types; leading mean response-window rate 53.77% despite sub-second average latency | Measures full-duplex participation and response timing; not a long-term storage-erasure benchmark |

Sources:
- 4DMemory: https://arxiv.org/abs/2610.32332
- EgoMemReason: https://arxiv.org/abs/2605.09874
- StreamArena: https://github.com/JIA-Lab-research/StreamArena
- OmniInteract: https://aclanthology.org/2026.emnlp-main.490/
- DIVE: https://arxiv.org/abs/2610.28405

### Competitive strategy

Reproduce 4DMemory and EgoMemReason using their official data/protocols where rights and access permit. Compare:
- standalone foundation models;
- recent-window and long-context baselines;
- summary-and-retrieve;
- event/episodic retrieval;
- specialist harnesses and model-specific RAG;
- Symthaea memory owner with evidence-ledger qualification.

Then apply our additional orthogonal challenge suite: source citations, exact as-of time, duplicate event identity, correction/supersession, clock-domain incomparability, common-cause dependence, revocation and post-revocation residual probes. Do not call this "better than 4DMemory" merely because a 12-episode synthetic set with a 15-span challenge measures additional properties. The claim requires official benchmark reproduction plus independent long-horizon adversarial evidence tests.

A meaningful next result would be non-inferior answer/event recall on 4DMemory/EgoMemReason at materially lower measured retained-memory and retrieval cost, while passing provenance/correction/revocation gates that the external protocol does not currently score.


## Evidence-qualified answer metric and evaluator hardening (21 November 2026)

The evaluator now publishes an additional **evidence-qualified answer accuracy** metric. It counts an answer only when the answer is correct, every required current source reference is present and admissible, provenance is complete, and the evidence contract has no violations. Raw answer accuracy, safe-decision accuracy, coverage, citation precision/recall and evidence-qualified accuracy remain separate. This prevents a lucky guess from being described as a successful evidence retrieval.

The synthetic four-condition reference tournament's known-answer evidence-qualified accuracies are:
- recent-window: 4/9;
- summary-only: 2/9;
- episodic retrieval: 6/9;
- evidence ledger: 8/9.

These figures are outcomes of hand-authored synthetic fixture policies, not learned memory systems or external benchmark scores.

Further hardening validates that duplicate deliveries preserve the exact immutable observation identity: modality set, content digest, capture tick, source dependency group, clock-domain ID and clock generation. The stream's availability ticks must be monotonically ordered. Derived-memory lineage is validated as an acyclic dependency graph; unknown source dependencies, unknown parents, temporal ancestry violations and cycles fail closed.

Purge receipts remain explicitly **candidate-self-reported**. A completed synthetic purge-claim check is not independent proof that bytes were erased from live storage, indexes, backups, caches or external providers. Real deletion qualification needs independent storage/provider receipts and post-revocation residual probes in an isolated evaluator.


## As-known-before-change checkpoint extension (21 November 2026)

The first nine episodes tested correction and revocation after those control events had become available. That left an important temporal-state gap: systems also need to preserve the previously valid state before a future correction or revocation arrives.

Two paired checkpoint episodes are now included, taking the synthetic fixture set from nine to eleven:

10. **Correction-before-arrival:** the initial machine-state observation is available at tick 0; a replacement observation arrives at tick 600; the correction becomes available at tick 601; the query is fixed at tick 300. The correct answer must use the initial observation as the current state. The later correction must not leak backward.
11. **Revocation-before-effect:** a private audio observation is available at tick 0, a public video action at tick 120, and a derived summary at tick 600; the revocation only arrives at tick 1800; the query is tick 1200. The private observation remains eligible at that earlier checkpoint. Once the revocation is available, the original post-revocation case still requires it and all derived descendants to become ineligible and be purged.

The timeline is a stored synthetic replay fixture, not permission to expose future events to the online candidate. A real candidate receives only the causal prefix through the current query tick. The evaluator may hold the full fixture and applies corrections/revocations only when their availability tick is at or before the prediction's as-of tick.

The exact-head fixture workflow now allows future control events to remain in the stored stream while verifying:
- stream availability order is monotonic;
- event identities are unique;
- correction source and replacement observations existed before the correction event;
- revocation target existed before the revocation event;
- every truth-required source observation was available by that episode's query tick.

The evidence-ledger reference known-answer score is now 11/12 answers, 12/12 safe decisions, 11/12 coverage, 12/12 complete provenance, zero false memories and zero hard evidence violations. The four-condition tournament reference summary has been updated for 12 matched episodes. All remain hand-authored synthetic policy checks, not model scores or blind-set results.

The paired checkpoints make temporal qualification stricter: a system may neither apply a future correction too early nor retain a revoked source after the revocation becomes effective. Both error directions matter.


## New spatial-instance memory comparison: EgoWitness (19 November 2026)

The external long-horizon landscape now includes EgoWitness (arXiv:2611.25849), a new instance-level spatial-memory benchmark. It reports 8,453 human-verified questions across about 324 hours, seven abilities, more than one million reviewed masks, 34 registered landmarks and 21 movable-object labels. The paper reports a 46.97% direct open-weight VLM maximum, with no model above 35% on the “who last saw this object?” task. Its trained EgoClue 4B controller reports 72.65% on 1,320 held-out week-long questions; evidence-trace synthesis reports 74.85% on that same week-long slice and 80.56% on a separate 787-question cross-environment slice.

These are separate task/system slices and must not be averaged or compared as one number. EgoWitness emphasizes stable physical-instance identity, cross-view and cross-wearer evidence, room anchors and visit reasoning. Our event-memory stream schema alone does not test those properties; a separate adapter and synthetic contract now lives in Symtropy issue #6574 / draft PR #6576.

References:
- EgoWitness paper: https://arxiv.org/abs/2611.25849
- Existing related benchmark owner: Symtropy #4414 (RoboCasa-Mem; mobile-manipulation memory is complementary, not the same protocol)

The EgoWitness media, masks and annotations are not imported by either PR. Exact upstream release revision, source-video rights, mask/annotation licence and derivative rights remain to be audited per artefact. Public paper facts do not authorize copying or training on the underlying recordings.

The competitive path is now: (1) reproduce official EgoWitness and EgoMemReason scores under their own protocols where the datasets are lawfully accessible; (2) compare against direct VLM, recent-window, summary, retrieval and specialist-tool agents with matched budgets; and (3) add our independent provenance, as-known time, false-merge/split, correction, revocation and causal-evidence checks. The latter enrich the qualification profile but do not replace the official benchmark score.


## Multi-segment chain recall (12th scenario)

The benchmark now includes lh-012-multisegment-15-source-chain: a two-hour synthetic stream containing 15 source observations spread across 70 minutes, with a query that requires reconstructing the full ordered maintenance sequence. The candidate must return the exact 15 source references in temporal order, not merely mention the right event or retrieve the right set in arbitrary order.

This is motivated by StreamArena's long-video benchmark: 189 questions require at least two distinct evidence segments, its most demanding questions require up to 15 segments, and some retrospection queries rely on evidence more than an hour earlier. The synthetic challenge isolates the evidence-chain property without importing the benchmark's media or assuming identical task semantics.

The evaluator now supports optional required_ordered_observation_refs in evaluator truth. When present, it must be a unique permutation of the required evidence set. The candidate's current_observation_refs must match that order exactly. An unordered but complete evidence bag receives current_observation_order_mismatch, loses answer correctness and cannot be marked provenance-complete.

The four reference lanes intentionally differ:
- recent-window: the 15 source observations have expired from the 20-minute window, so the policy abstains;
- summary-only: it retrieves all 15 references but returns them in reverse order;
- episodic retrieval: it retrieves only 14 of the 15 required spans;
- evidence ledger: it retrieves all 15 references in the correct order.

The resulting synthetic reference scorecard is now 12 episodes: the evidence-ledger policy produces 11/12 answer correctness, 12/12 correct decisions, 11/12 coverage, complete provenance and zero hard violations. The four-way tournament expected summary is updated for all 12 matched episodes. These are hand-authored reference-policy checks, not model results or comparative evidence against StreamArena.


## Closed-loop evidence lifecycle: cross-benchmark integration

The longer-term paradigm is to qualify a continuous system as one evidence lifecycle, not a collection of independent perception demos. The repository now has separate benchmark owners for each stage:

| Lifecycle stage | Existing owner | Qualification question |
|---|---|---|
| Decide what to observe | Symtropy #6222 / #6225 and active-sensing PR #6549 | Does the next observation have positive decision value after cost, latency and privacy? |
| Decide which sensor path matters | Modality necessity PR #6307 | Does audio, video or their combination actually change the answer on matched counterfactuals? |
| Acquire and preserve observations | Symtropy #6201, camera adapter #6302, audio adapter issue #6339 | Are source bytes, availability, capture time, transport duplicates/drops and clock identity preserved? |
| Retain and retrieve evidence | Symtropy #6357 / PR #6359; Symthaea #10644, #9075 and #9177 | Can the system reconstruct exact events and source spans after long delays, corrections and distraction? |
| Track physical identity | EgoWitness adapter PR #6576 | Does the same physical instance remain distinct from lookalikes across rooms, days and wearers? |
| Decide whether and when to respond | Response-window PR #6433 | Does it speak only when useful, within the valid response window, and stop on cancellation? |
| Revoke and qualify residuals | Post-revocation PR #6489 | Do revoked source facts and every derived descendant become ineligible, with residual probes checking for information leakage? |

### Shared invariants for future end-to-end runs

1. **Immutable source identity:** one observation reference cannot silently change its content digest, source interval, clock domain/generation or dependency lineage.
2. **Two times, never one:** capture/measurement time and availability/receipt time remain distinct. A system can only use evidence at or before its decision cutoff.
3. **Causal lineage across stages:** active sensing requests, sensor deliveries, memory items, answers and playback events carry stable parent/child identities. A downstream summary must not invent stronger provenance than its sources.
4. **No double-counted corroboration:** two modalities or viewpoints from the same dependency root remain dependent evidence unless an independent source is established.
5. **Correction is bitemporal:** preserve what was believed at each earlier cutoff while applying a correction only after it becomes available.
6. **Revocation is transitive:** source revocation invalidates derived summaries, embeddings, indexes, caches and response context; candidate self-reports are not treated as independent proof of physical deletion.
7. **Response is an evidenced action:** controller permission is not playback; renderer-confirmed start/end and stop receipts qualify the actual intervention interval.
8. **Independent qualification:** candidate processes never receive evaluator truth. Public fixture separation is for mechanics only; blind claims need separately controlled media, truth, credentials and logs.

A future full-loop episode should therefore look like:

**uncertainty → request an observation → receive an immutable sensor event → update/revise evidence memory → retrieve a source-linked answer → respond or remain silent → stop/cancel if superseded → propagate later corrections or revocations.**

The per-stage benchmarks remain separate and keep their own official metrics. An end-to-end report should publish the joint trace and a Pareto profile—task quality, coverage, temporal correctness, evidence compliance, false intervention, long-horizon retrieval, p95 latency, input bytes, memory bytes, compute/energy—rather than one weighted score that can conceal a failure at an earlier stage.

## A more ambitious benchmark ladder

After reproducing the official external protocols, expand the synthetic 15-span chain along axes that can falsify brittle memory strategies:

- **Span count:** 2 / 5 / 15 / 30 distinct evidence intervals; measure exact set and order recovery.
- **Retrospection gap:** 1 / 10 / 60 minutes / 4 hours / 24 hours / 7 days.
- **Entity cardinality:** one actor/object, then many similar instances, then cross-wearer and cross-room continuity.
- **Evidence topology:** linear chain, branching causal event, contradictory sources, shared common cause, corrected history and revoked ancestor.
- **Memory pressure:** fixed-byte budgets, forced eviction, summary compression and retrieval latency measured under the same stream.
- **Intervention:** proactive response-required events, expected silence, overlapping opportunities, user interruption and cancellation.

Proposed research acceptance targets, not achieved results:
- at least 95% exact ordered-chain retrieval on a powered held-out 15-span cohort;
- zero future-source, superseded-current-state or revoked-source qualification;
- zero silent duplicate-event inflation;
- zero false instance merges on explicit same-category/lookalike controls;
- non-inferior ordered evidence recall under a preregistered memory-budget reduction;
- improve DIVE-style response-window correctness at matched p95 latency;
- publish a complete trace from observation request to renderer stop, with independent digest and exact-head receipts.

The 15-span synthetic episode validates only one finite sequence contract. Official StreamArena, DIVE, OmniInteract and EgoWitness scores must still be reproduced independently using their own splits, annotations and protocols. No benchmark numbers are transferred or conflated.


## New long-horizon and adaptive-access benchmark anchors (checked 21 November 2026)

- **StreamArena (August 2026):** 243 full-length videos averaging 88.8 minutes and 3,646 open-ended question-answer pairs across real-time perception, historical retrospection, proactive interaction and tool use. Its paper identifies the trade-off between recent-window-only, text-summary and repeatedly compressed visual memory. https://arxiv.org/abs/2608.05703
- **SEW-Bench / Budget-Constrained Embodied Perception (24 August 2026):** under a 4,096-token decision budget, access-structured perception reports 75–94% episodic retrieval versus 3–19% for equal-budget query-independent sampling. Crucially, the authors also report that the full three-component architecture did not beat the verbatim-only baseline and two preregistered falsification criteria fired. This is an important warning against assuming compression is automatically beneficial. https://arxiv.org/abs/2608.22975
- **TRACE (25 September 2026):** 1,240 records from 517 videos and explicit reporting of temporal validity, history processing, response timing, false alarms, missed windows, workload, completion and reliability. https://arxiv.org/abs/2609.30670
- **SPOT-Bench (NeurIPS 2026 E&D):** proactive streaming benchmark at 1Hz with a posted Ctrl-SPOT T-F1@5 overall baseline of 20.9; every prediction is evaluated, including non-response dead-time. https://dibschat.github.io/SPOT-Bench/

These results suggest two distinct research priorities: query-conditioned evidence access rather than query-independent frame sampling, and exact evidence retention rather than lossy summary recall. But our benchmark must preserve the negative SEW-Bench ablation result: test verbatim-only and compression-only controls. The objective is not "build a sophisticated memory stack"; it is to establish which components earn their memory and compute costs under matched budgets.
