# AV Evidence Gain Trajectory Ledger v0

**Issue:** Symtropy #6895  
**Stacked on:** #6856 / draft PR #6859 (claim-level evidence certificates)  
**Composes:** #6753 / #6750 (all-action EVSI and decision regret), #6564 / #6555 (event-by-event executor), #6847 (inferential Pareto), #6838 (rank confidence sequences), #6754 (blind custody), Symthaea #13179 (candidate trace export).

## Objective

Measure what changes in a candidate's observable claim certificates after each acquisition, and join those changes to the exact pre-outcome action forecast and independently scored action-value receipt.

The ledger reports a vector, not a blended reward:
- newly grounded claims and lost-grounded regressions;
- external user-simulator / human feedback events, separate from sensor observations and their media-grounding score;
- per-claim citation precision/recall/F1 deltas;
- temporal and spatial IoU deltas where annotated;
- newly delivered observations cited versus uncited;
- linked #6753 decision regret, missed-best-action and feasibility;
- per-turn resource deltas and measured energy only when externally evidenced.

An uncited observation is not automatically redundant: candidate citations establish what the candidate claims to support an answer, not what its private internal representations used.

## Dependency / stacking

This PR is intentionally stacked on the current research/av-evidence-certificate-v0 head because it imports the #6859 evaluator as a shared scorer. It does not vendor or reimplement that scorer. The eventual parent #6859 must merge before this PR can be retargeted to main.

The runner's candidate-visible workload/submission remains turn-local. evaluator_source_catalog is evaluator-only and contains immutable source records for the whole episode, including observations that were never delivered. It lets hidden truth annotate an event that an attempted acquisition failed to deliver without ever exposing that record to the candidate.

## Normative event sequence

For each turn:
1. Candidate receives a delivered-observation workload with transcript head and decision cutoff.
2. Candidate commits a forecast for every currently eligible #6753 action plus __stop__, preserving action ID, modality, description, cost, arrival tick, source-dependency group and information-gain proxy. The forecast is bound to episode ID, turn ID, transcript head, current logical tick, deadline, cost weight and budget.
3. Candidate selects one forecasted action or stop.
4. Executor attempts that selected action. The next turn identifies the same action as preceding_action_id, records its result status and includes only observations actually delivered since the prior cutoff.
5. The ledger verifies that new observation IDs exactly equal the catalog growth, that earlier observation metadata is immutable, and that each new observation arrived after the prior decision cutoff.
6. The existing #6859 scorer is run on the current as-known workload/submission, using the immutable evaluator-only full source catalog only for truth validation.
7. The linked #6753 receipt is joined by episode, turn, policy, transcript head and forecast digest. Its action regret and missed-best-action fields are copied, not recomputed.
8. The completed trajectory must end with __stop__; no turn may follow a stop decision.

### External feedback is a separate event plane

A turn may carry external feedback events with a unique ID, source role (user simulator, human user or operator), kind (hint, clarification, correction, challenge or request), delivered tick, content digest and parent transcript head. These events may arrive alone or alongside a sensor observation result. They update the candidate-visible transcript but are **not** inserted into the audio/video observation catalog and are never counted as independent sensory evidence.

A DELIVERED action outcome must add at least one sensor observation or one valid external feedback event. A failed/timed-out/malformed outcome may add neither. The feedback message body is retained in the candidate trace/input archive; the ledger carries its digest and timing so it can be replay-bound without treating a user hint as ground truth. Any answer change after feedback is measured as an observable per-claim state transition, not assumed to be causally explained by the feedback.

Every candidate turn sees only its own delivered catalog. An observation in the evaluator source catalog is not candidate-visible until it appears in a later turn's workload.

## Identity and wire contract

The bundle binds:
- trajectory, episode and source-cluster identity;
- policy, model, runtime, configuration and hardware digests;
- the evaluator-only full source observation catalog and immutable truth;
- ordered turns with turn ID/index, parent transcript head, predecessor action, action-result digest/status and exact new observation IDs;
- per-turn #6859 workload and submission;
- per-turn complete all-action forecast and linked #6753 scorecard receipt;
- per-turn resource delta.

Forecasts and receipts carry both episode ID and turn ID. The action catalogue follows #6753's full workload shape; cost weight and budget are committed before outcomes. The forecast hash is calculated over the exact canonical forecast record; the receipt must bind to that digest and the same transcript head. This distinguishes a multi-turn forecast from a single episode-level #6753 scorecard.

## Failure and resource handling

- DELIVERED must add at least one new observation.
- FAILED, TIMEOUT and MALFORMED may add no observations; they remain represented as attempted action outcomes with a required result digest.
- Each non-initial turn carries the previous turn's transcript head as its parent. Initial turns have no parent head and no action-result digest.
- External feedback events are accounted separately by source role and feedback kind; they are not counted as sensor observations.
- Missing resource values remain null and have observed counts, not zeros.
- NOT_MEASURED energy must have null value/method. MEASURED energy requires a meter method and evidence digest.
- Peak RAM/VRAM remain per-turn high-water metrics and are not summed as additive consumption.
- No energy or compute-saving claim is made from synthetic resource fixtures.

## Running

From the repository root:

    python3 -m json.tool schemas/evidence-gain-trajectory-v1.schema.json > /dev/null
    python3 -m unittest discover -s tools/av-evidence-gain-trajectory -p 'test_*.py' -v

The evaluator uses the standard library plus the existing #6859 evaluator available from the stacked parent branch:

    python3 tools/av-evidence-gain-trajectory/evidence_gain_ledger.py --input trajectory-v1.json --output evidence-gain-scorecard.json

## Known-answer / adversarial tests

The current suite tests:
- a short, independent alarm observation newly grounding an abstained claim;
- a user-simulator hint arriving without a sensor observation, preserved as an external feedback event rather than evidence;
- exact action/receipt and transcript binding;
- complete action forecasts including stop;
- catalog growth, failed acquisition and failed-without-delivery semantics;
- immutable observations and rejection of catalog shrinkage;
- no turn after stop and mandatory stop termination;
- hidden future/un-delivered truth records never appearing in candidate workload;
- resource provenance and measured-energy evidence requirements;
- deterministic replay.

These are evaluator mechanics tests, not model results or blind benchmark outcomes.

## External comparison and benchmark targets

This ledger is intended to be evaluated against current agentic video systems and benchmarks, not only fixed-input video QA.

### Direct method comparator: Open-o3-Video-v2

Open-o3-Video-v2 (October 2026) is the closest published method comparator: it starts from sparse observations, requests additional video crops, grounds retrieved moments/objects, and uses Clue-Seek GRPO to reward clue coverage and useful evidence gain while penalizing repetitive, fragmented or uninformative retrieval. Its official repository reports **8B: 40.7 mAM / 56.5 mLGM on V-STaR**; the paper's abstract reports **4B: 40.4 mAM / 56.3 mLGM**, both under its own benchmark protocol.

Paper: https://arxiv.org/abs/2610.17656  
Official code / evaluation recipes: https://github.com/Multimedia-Semantic-Analytics-Lab/Open-o3-Video-v2  
Project: https://msalab-pku.github.io/projects/Open-o3-Video-v2

Reproduce the native V-STaR metric and the paper's nine-benchmark evaluation on the exact released 4B/8B revisions where feasible. Preserve source benchmark licenses; the Open-o3 code is Apache-2.0, but video datasets retain their original licenses. Do not compare our claim-certificate metrics directly to mAM/mLGM.

### New confirmatory target: VideoACE

VideoACE (18 November 2026) contains 1,000 multiple-choice questions across six fine-grained high-frame-rate action categories, with temporal evidence annotations and user hints screened for answer leakage. Its environment supports temporal coverage, sampling density and spatial-detail actions, plus a user simulator that can provide feedback. It evaluates three settings: single-pass, autonomous tool use, and tool use with user feedback.

The trajectory ledger now keeps feedback in a separate external-event lane, so a user hint can change the candidate transcript and answer without being misrepresented as an independent camera/microphone observation.
Paper: https://arxiv.org/abs/2611.24184

Use this as the priority test of the trajectory ledger once the official benchmark package and tool protocol are accessible. Compare:
1. single-pass A3/A4 baseline;
2. autonomous acquisition A5;
3. A5 with user feedback / requested revision;
4. uniform and random budget-matched sampling controls.

Treat simulated user feedback as a separate source with its own timestamp and provenance; do not merge it with sensor acquisition or score the same hint as if it were an independently measured observation. The ledger should show whether feedback changed a claim, what new observation followed, and whether the final certificate remained supported.

### Complementary general-agent benchmarks

- **VideoGAIA:** 271 model–human co-designed tasks; multi-turn, tool-augmented video interaction and evidence integration. The preprint reports frontier models including GPT-5.5 and Kimi-K3 below 60% accuracy. This is useful for general tool-using video tasks, but it is not the same workload as high-frame-rate transient-action perception. https://arxiv.org/abs/2608.14718
- **MVX-Bench / SAMA:** 1,442 questions over 4,255 videos, spanning 11 classical computer-vision task families, with cross-video identity matching and conflict-aware verification. This is a future multi-video memory extension; the current v0 ledger is single-episode. https://aclanthology.org/2026.findings-emnlp.429/
- **VideoWebArena:** 2,021 web-agent tasks based on 74 tutorial videos (~4 hours), split into skill retention and factual retention. It is an older, complementary task-completion benchmark, not a direct high-FPS leaderboard. https://videowebarena.github.io/

### Our research differentiation

Open-o3-Video-v2 already trains for stepwise evidence gain. Our target is therefore not to claim novelty for the idea of rewarding useful retrieval. The distinct contribution we are testing is an independently replayable **forecast → action → delivered observation → claim-certificate change → counterfactual necessity → resource cost** record, joined to #6753 action regret and #6859 as-of/citation checks.

Targets (not achieved):
- reproduce Open-o3-Video-v2's own V-STaR mAM/mLGM and nine-task protocol;
- reproduce VideoACE's single-pass / autonomous / feedback conditions, with the user's feedback source separately accounted;
- achieve non-inferior grounded answer quality at lower observation/tool cost than uniform and random budget-matched policies;
- show that deleting irrelevant evidence preserves answers, while removing all sufficient evidence forces appropriate revision or abstention.

Do not claim superiority until candidate revision, source split, tool budgets, sampling protocol and decision deadlines are matched.

## Explicit v0 limits

- One trajectory/episode per bundle; cross-episode aggregation belongs in a separate report layer.
- The ledger joins an external #6753 scorecard receipt by digest; it does not recompute potential outcomes, EVSI or optimal action.
- It uses #6859 claim certificates; it does not judge semantic entailment or inspect private model reasoning.
- A citation is not proof of internal causal use; uncited observations are not automatically redundant.
- The executor must supply honest append-only action/result/observation records; this ledger does not run the AV system.
- The public parent fixtures are synthetic. A blind confirmatory run requires #6754's independent custody.

## Claim boundary

This ledger measures observable evidence-certificate deltas and joins them to pre-outcome action-value outputs. It does not establish internal reasoning, causal sufficiency beyond evaluator annotations, physical resource use, or SOTA performance.
