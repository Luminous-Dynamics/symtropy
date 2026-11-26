# AV-OBSERVATION-001 decision-sufficiency bridge v0

**Status:** adapter contract sketch; no implementation or benchmark result claimed.  
**Scenario owner:** #6183 / #1595.  
**Candidate one-step policy:** #7555 / #7558.  
**Independent claim evaluator:** #7577.  
**Sequential acquisition tournament:** #6675.  
**Memory cost/regret frontier:** Symthaea MACE #13284 / #13261.  
**Streaming spatial consumer:** Symthaea #12837.

## Purpose

Connect the spatial decision-sufficiency cases in `scenario-matrix-spatial-decision-v0.tsv` to the existing decision-partition policy and evaluator. The bridge is deliberately an adapter contract, not a new learner, spatial map, action executor, episode runner, posterior, scheduler, evidence graph or truth store.

The question is not whether the candidate reconstructs one uniquely correct map. It is whether the candidate preserves every map alternative that is admissible under the current public evidence prefix and correctly determines whether those alternatives imply the same downstream decision.

## Candidate-visible projection

For each frozen evidence prefix and declared DecisionNeed, the candidate adapter projects only the candidate's own current map hypotheses and permitted probe catalogue into the existing Symtropy #7558 interchange types:

- `DecisionHypothesisProjectionV0`: candidate-local hypothesis ID and candidate-predicted downstream decision-class ID.
- `DecisionProbeOptionV0`: declared probe ID with acquisition cost and latency.
- `DecisionProbeOutcomeSupportV0`: candidate-predicted set-valued possible outcomes for each (probe, candidate hypothesis).
- Remaining resource budget and the candidate's frozen policy result.

Decision-class IDs are candidate-local labels. They are not evaluator witness IDs, map IDs, stable world identifiers or a new ontology. Every hypothesis and probe projection must bind to the exact upstream evidence-prefix digest and policy/adapter revision in the owning episode receipt. The existing owner/episode protocol remains responsible for custody, permission, deadlines and process separation.

## Evaluator-only projection

The independent owner/evaluator holds separately:

- the complete admissible map-witness set for the exact prefix;
- evaluator decision-equivalence classes under the frozen downstream utility/action contract;
- the structural-validator binding from candidate-local hypothesis IDs to canonical witness identities;
- actual set-valued probe outcome support for each declared probe and each admissible witness.

These fields must not appear in candidate-visible JSON, logs, environment variables, filenames, shared memory, prompts or candidate receipts before the freeze boundary. Missing witness coverage is not evidence that the omitted alternative does not exist.

The evaluator uses #7577 to score false exclusion/inclusion, false decision merges/splits, missing/spurious outcome support, false decision-sufficiency, actual decision-disagreement reduction and unnecessary abstention. It must reject undeclared probe IDs.

## Expected behavior by fixture family

| Fixture family | Candidate behavior | Independent evaluator check |
|---|---|---|
| Decision-equivalent maps | Preserve multiple map explanations, but declare the shared downstream decision sufficient and stop further structural probing | Complete witness coverage; one evaluator decision class; valid sufficiency receipt |
| Decision-changing map | Do not declare sufficient while an admissible blocked-route or alternate-action witness remains | Omitted-witness and false-sufficiency checks |
| Targeted spatial probe | Propose only a legal, budget-feasible probe whose possible outcomes may separate the decision classes | Actual truth supports must demonstrate reduction; candidate-predicted supports are not trusted |
| Denied/late probe | Do not treat a denied or post-deadline observation as available evidence | Permission/deadline eligibility remains with #6675 / episode owner; no fictitious observation is scored as received |
| Summary-lineage failure | Do not infer direct adjacency from a summary that only supports sequential traversal | Source-span/evidence lineage and hidden map truth; summaries cannot create new physical observations |
| Late correction | Revise or retract the affected relation while preserving what was known at prior query times | Bitemporal correction-at-use and revision receipt; MACE/source-event owners remain authoritative |

## Owner handoff

1. **#12837 / Symthaea** produces causal-prefix spatial and audiovisual hypotheses with source intervals, clock/pose uncertainty, revisions and modality provenance.
2. **This adapter** translates candidate map hypotheses, downstream decision labels, candidate probe supports and available budgets into #7558 types. It must not introduce another scheduler.
3. **#7558** proposes a one-step decision-sufficient probe or stop/abstain disposition. It does not execute the probe.
4. **#7577** independently qualifies the proposal against evaluator-held map witnesses and actual outcome supports. It does not execute the probe or measure real-world physical accuracy.
5. **#6675** owns sequential acquire/wait/repeat/stop/answer/abstain tournament semantics, arrival/deadline eligibility, resource accounting and exact finite-horizon policy oracle.
6. **MACE #13284** evaluates downstream utility/regret and memory cost–regret frontiers. A reduction in disagreement-pair count is not itself a utility or safety guarantee.

## Required negative controls

- Remove one admissible blocked-route witness while leaving all candidate-visible data unchanged: the evaluator must reject false sufficiency.
- Merge two evaluator-distinct decision classes under one candidate class: false-equivalence count must increase.
- Split evaluator-equivalent maps into separate candidate classes: false-split count must increase, but no false-safe result is allowed.
- Add a candidate outcome prediction not supported by evaluator truth: the spurious-support count must increase.
- Remove one actual outcome from the candidate prediction: the missed-support count must increase.
- Add evaluator truth for an undeclared probe: validation must fail closed.
- Change only evaluator world IDs or reorder hidden truth rows: candidate-visible serialization and candidate action must remain unchanged.
- Change the candidate's declared decision mapping while preserving map witnesses: the evaluator must recompute the resulting partition rather than trust the label text.
- Make all remaining candidate probe options unaffordable, unauthorized or late: the policy must not claim a probe can execute.

## Claim boundaries

- A valid finite decision-sufficiency receipt does not prove the map is physically correct, complete beyond the admitted witness set, or safe under an unmodeled world.
- A candidate map hypothesis is not a hidden world witness.
- A proposed probe is not an executed observation.
- A predicted outcome support is not evaluator truth.
- Fewer unresolved map hypotheses is not necessarily better; decision-equivalent alternatives may be retained.
- Fewer disagreement pairs is not necessarily lower downstream regret; utility and cost qualification stays with MACE and the sequential tournament.
- Passing synthetic fixtures does not establish sensor calibration, physical localization, causal discovery or SOTA.

The first implementation should remain a pure adapter over existing types and owner contracts. Do not add a new service until a concrete missing capability is demonstrated by a failing test.
