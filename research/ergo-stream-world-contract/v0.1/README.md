# ErgoStream World Contract v0.1

**Date:** 2026-11-23  ·  **Status:** research contract with initial finite executable components; exact-head qualification pending
**Tracking:** [ERGO-STREAM-WORLD-001 #7149](https://github.com/Luminous-Dynamics/symtropy/issues/7149)
**Learner contract:** [Symthaea #13401](https://github.com/Luminous-Dynamics/symthaea/issues/13401)

## 1. Purpose and boundary

Symtropy provides the evaluator-owned half of a task-agnostic structural-acquisition experiment. It generates hidden worlds, emits causally available modality-native observations, records bounded actions and replay, freezes acquisition, and independently probes the learner's returned structural-library snapshot.

This package must not become a second cognitive learner, generic simulator, sensor driver, evidence authority, PhysicsIR, or world-model benchmark registry. Reuse SIM-REL episode/process and model-proposal contracts where their semantics fit.

## 2. Ownership split

| Concern | Owner |
|---|---|
| Hidden generator, world state, seeds, event/relationship truth | Symtropy trusted owner/evaluator |
| Causal stream delivery and permitted action accounting | Symtropy episode adapter |
| Unit/relation discovery and acquisition policy | Symthaea child agent |
| Structural-library format and epistemic lifecycle | Symthaea |
| Withheld structural probes, scoring and counterexamples | Symtropy evaluator |
| Claim qualification and domain promotion | Existing independent assurance owners |

## 3. Episode protocol

```text
owner creates hidden world from private recipe/seed
  -> creates public episode offer
  -> emits frames only when causally available
  -> child requests bounded observations/actions
  -> owner records every accepted/rejected attempt
  -> owner closes acquisition at declared budget/horizon
  -> child submits immutable library snapshot + lineage
  -> evaluator runs withheld probes/interventions
  -> owner emits scorecard and replay receipt
```

Acquisition and probe phases are isolated. The submitted snapshot is immutable for the primary score. A subsequent-learning experiment receives a new episode identity and cannot rewrite the first result.

## 4. Policy-visible information

The policy may see:

- an opaque episode token and contract version;
- public modality, action, clock, payload and cost semantics;
- frames that have arrived, with sequence number, measurement support and arrival time;
- explicit dropped-frame, overflow, discontinuity, decode-failure and missingness notices;
- its own action history and remaining declared budgets;
- public response status for accepted/rejected actions.

The policy must not see:

- generator source/configuration, private seed or hidden truth;
- latent state, exact boundary positions or hidden unit/relation/motif IDs;
- structural family label when it exposes the answer, hidden train/test category or evaluator eligibility;
- future frames, future observations not yet acquired, withheld probe questions or answer keys;
- evaluator score components or selection decisions during acquisition.

Process separation is not equivalent to OS sandbox isolation. A blind-evaluation claim requires the qualified runner/sandbox boundary; a local parent/child process bridge alone must be labelled process-separated only.

## 5. Frame semantics

Every delivered frame must preserve:

- stream/channel identity and monotonic sequence number;
- measurement support start/end and arrival time as separate values;
- clock-domain identity and declared uncertainty;
- modality-native payload schema and immutable payload digest/reference;
- source/world/episode identity only at the public opaque level;
- explicit missingness, truncation, dropped-frame and discontinuity markers.

Do not reorder by arrival time and then pretend this is measurement order. Do not interpolate gaps, coerce unknown time units, flatten all modalities into one vector, or silently repair malformed payloads. Request time and action execution/result time are separate timestamps.

## 6. Action semantics

V0 should begin with:

1. `next_available`: consume the next causally available frame;
2. `request_window`: request a bounded prior/current window from an admitted channel, with explicit window limits and cost;
3. `focus_channel`: allocate a bounded higher-resolution/feature view where the generator explicitly provides it;
4. `apply_probe`: execute a world intervention only for generator families whose exact effect, cost and safety semantics are defined.

All action requests—including unknown, malformed, rejected, over-budget and timed-out requests—remain in the attempt denominator with a disposition. The provider must not let a policy call arbitrary owner functions or inspect world memory.

## 7. Deterministic generator ladder

Begin with a tiny discrete family whose full truth is enumerable:

- latent token/event alphabet with hidden unit segmentation;
- ordered, overlapping, recurrent and nested motifs;
- repeated motifs with different noise and nuisance transformations;
- matched-marginal but different higher-order structures;
- negative controls with no shared grammar;
- prefixes that are observationally equivalent but separable by one admitted probe.

Next add cross-modal association and bounded 2D rigid scenes. Do not begin with raw unbounded audio/video, continuous PDEs, real robot control or a purported universal physical generator.

Every generator family must define:

- exact hidden truth and equivalence classes;
- policy-visible observation process and any sensor/channel nuisance;
- action set, effects, costs and time semantics;
- train/dev/confirmatory split by world/grammar/composition;
- exact tiny-instance oracle and independent known-answer fixtures;
- failure and non-identifiability controls;
- source, schema, generator, evaluator and payload identities.

## 8. Post-acquisition probes

The evaluator should distinguish:

- unit boundary recovery from unit identity recovery;
- relation recovery from predictive correlation;
- motif recognition from new composition generalization;
- nuisance invariance from false invariance under a real rule change;
- observational equivalence from interventional equivalence;
- structural prediction from downstream decision quality;
- correct abstention from failure to produce an answer.

Scoring is performed against evaluator-owned equivalence classes, not candidate labels or string equality. A syntactic variant may be equivalent; two programs with the same finite observed outputs may remain non-identifiable.

## 9. Replay and evidence

Bind exact identities for:

- owner generator/evaluator commit and schemas;
- hidden recipe/seed commitment held by owner;
- public episode offer and delivered frame sequence;
- action requests and dispositions;
- child executable/configuration and structural-library snapshot;
- post-acquisition probe set and result payload;
- completeness status and all denominators.

Reuse repository-standard authenticated SHA-256 / receipt authority. Do not use FNV or a home-grown digest for authenticity. Keep raw seeds, hidden truth and evaluator keys out of public receipts unless a later release policy explicitly allows publication after the blind period.

Any dropped frame, overflow, missing response, process failure or incomplete receipt produces an explicit incomplete run; it must not be silently scored as a successful or complete episode.

## 10. Qualification gates

1. Independent known-answer evaluator agrees with the tiny generator on exhaustive worlds.
2. Serialized policy views contain no hidden labels, seeds or future frames.
3. Shuffling timestamps, symbols and cross-modal alignment changes only the expected score cells; leakage negatives are detected.
4. Deterministic replay reproduces delivered frames, action dispositions and evaluator payloads exactly within declared numeric semantics.
5. Missing, delayed, contradictory and uninformative observations remain represented in all denominators.
6. Process separation, OS sandbox isolation, deterministic replay, authenticity and physical validity are claimed separately.
7. No SOTA claim until strongest compatible baselines run under matched observation, compute, latency and memory budgets.

### Current finite multimodal slice — implementation staged, execution pending

The initial two-modality categorical microcosm is implemented in `src/multimodal.rs` on the stacked research branch for #7442. It is deliberately narrower than the full protocol:

- Separate modality-local observation IDs, sensor timestamps and arrival ticks.
- Independent affine clock models with offset, rate, jitter and bounded arrival delay.
- Missing observations with fixed per-modality marginal-preserving generation.
- Aligned positive and marginal-matched shuffled negative regimes. Independent PRNG substreams preserve each modality's event-symbol and timestamp marginals across those two regimes, including the declared dropout masks.
- A periodic non-identifiable regime with four admissible phase correspondences and no shared clock anchors.
- Separate evaluator functions for event-correspondence claims and affine clock-map residuals.
- Unique / alternatives / unresolved claim dispositions; an unjustified singleton is explicitly scored as false-unique when the evaluator retains multiple admissible correspondences.
- Causal policy views filter by arrival tick and order by arrival, never by sensor timestamp.

The first version uses explicit shared calibration pulses as an optional independent clock-identifiability factor. It is not a general blind clock-estimation algorithm. Periodic event ambiguity can be tested both with anchors (identifiable clock, ambiguous correspondence) and without anchors (both unresolved). No semantic or causal labels are serialized to the policy view.

Eight focused tests cover aligned positives, exact matched marginals under dropout, shuffled negative correspondence, periodic ambiguity with and without shared clock anchors, clock/semantic score separation, causal arrival prefixes, explicit out-of-order sensor-time delivery, and invalid configuration. The existing `ErgoStream microcosm` workflow is the execution authority; no test, doctest, Clippy or benchmark result is claimed until it passes on the exact branch head.

## 11. First executable milestone

Extend the existing SIM-REL blind episode contract with a typed adapter for one discrete motif-stream world. Add one independent exact evaluator and one minimal child policy. Before adding large modalities, prove hidden-state non-exposure, exact replay, denominator conservation and held-out composition scoring.

## 12. Non-claims

An executable synthetic stream world is not a model of human learning, a biological ergobrain, a universal learning system, proof of semantic understanding, consciousness evidence, or a physical-world truth oracle. It is a controlled test bed for the stated structural-acquisition hypotheses.