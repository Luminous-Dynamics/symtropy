# AV Evidence Gain Trajectory Ledger v0

**Issue:** Symtropy #6895  
**Stacked on:** #6856 / draft PR #6859 (claim-level evidence certificates)  
**Composes:** #6753 / #6750 (all-action EVSI and decision regret), #6564 / #6555 (event-by-event executor), #6847 (inferential Pareto), #6838 (rank confidence sequences), #6754 (blind custody), Symthaea #13179 (candidate trace export).

## Objective

Measure what changes in a candidate's observable claim certificates after each acquisition, and join those changes to the exact pre-outcome action forecast and independently scored action-value receipt.

The ledger reports a vector, not a blended reward:
- newly grounded claims and lost-grounded regressions;
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
- exact action/receipt and transcript binding;
- complete action forecasts including stop;
- catalog growth, failed acquisition and failed-without-delivery semantics;
- immutable observations and rejection of catalog shrinkage;
- no turn after stop and mandatory stop termination;
- hidden future/un-delivered truth records never appearing in candidate workload;
- resource provenance and measured-energy evidence requirements;
- deterministic replay.

These are evaluator mechanics tests, not model results or blind benchmark outcomes.

## External comparison and target

Open-o3-Video-v2 (October 2026) is a direct method comparator: iterative crop seeking, explicit temporal/object/box grounding and stepwise clue-coverage/evidence-gain rewards. Its reported 4B V-STaR results are 40.4 mAM and 56.3 mLGM, +8.5/+12.6 over Qwen3-VL-4B in the paper's protocol.

Paper: https://arxiv.org/abs/2610.17656  
Project: https://msalab-pku.github.io/projects/Open-o3-Video-v2

Our proposed difference is the complete external audit trail from forecast → action → delivered observation → claim-certificate change → counterfactual necessity → resource cost, with #6753 independent all-action regret and #6859 source-cutoff validation. We still need to reproduce V-STaR and run matched backbone/resolution/tool-budget conditions before claiming superiority.

## Explicit v0 limits

- One trajectory/episode per bundle; cross-episode aggregation belongs in a separate report layer.
- The ledger joins an external #6753 scorecard receipt by digest; it does not recompute potential outcomes, EVSI or optimal action.
- It uses #6859 claim certificates; it does not judge semantic entailment or inspect private model reasoning.
- A citation is not proof of internal causal use; uncited observations are not automatically redundant.
- The executor must supply honest append-only action/result/observation records; this ledger does not run the AV system.
- The public parent fixtures are synthetic. A blind confirmatory run requires #6754's independent custody.

## Claim boundary

This ledger measures observable evidence-certificate deltas and joins them to pre-outcome action-value outputs. It does not establish internal reasoning, causal sufficiency beyond evaluator annotations, physical resource use, or SOTA performance.
