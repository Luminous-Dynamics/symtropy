# SPATIAL-STATE-INTERVENTION-001 — SpaceCast crosswalk and protocol

**Status:** metadata-only protocol and development controls. No media, dataset or model output is included.

Parent: [Symtropy #7928](https://github.com/Luminous-Dynamics/symtropy/issues/7928)  
Research benchmark: [SpaceCast-Bench, arXiv:2610.12402](https://arxiv.org/abs/2610.12402)

## 1. Research question

Can an agent move from *recognizing relations in an observed scene* to *maintaining and updating a spatial state under interventions that have not yet been observed*?

We treat the task as an observe–transform–infer pipeline:

`O_{0:t} \rightarrow S_0 \rightarrow T \rightarrow S_1 \rightarrow q(S_1)`

- `O`: candidate-visible observations, with capture and availability semantics.
- `S0`: candidate spatial state (possibly set-valued or ambiguous).
- `T`: typed local or global intervention.
- `S1`: predicted state after the intervention.
- `q`: relation, visibility, distance, direction, attachment or downstream decision query.

The key new score is not merely post-state accuracy. It is the **relational delta**: identify which claims must change, which must remain invariant, which become unknown, and which prior claims must be retracted when new evidence arrives.

## 2. Why this benchmark matters

SpaceCast-Bench reports 3,862 questions from 182 indoor scenes across 16 task types: 1,368 L1 static, 1,709 L2 local prediction and 785 L3 global prediction questions. It reports 58.0% for its highest-scoring tested base model (Gemini 3.5 Flash), 55.3% for Qwen3.7-Plus, 39.3% for its best listed open-weight general model (Qwen3.5-27B), 25.7–29.1% for listed spatial-specialized models, 25.9% random and 87.2% human. See the paper's tables and benchmark construction at the linked source.

The paper's diagnosis matters more than its leaderboard number:
- several leading general models lose substantial accuracy from static perception to local prediction;
- object-centric direction is a particularly weak reference frame;
- bridge views materially help connect spatially separated observations;
- text/BEV 3D evidence can help general models, while generated post-intervention images may contain object deformation, scale or perspective errors;
- the reported 65.7% staged-SFT Qwen3-VL-4B result is a **trained** arm using generated training examples from scenes disjoint from the benchmark. It is not a zero-shot comparator.

This is an unusually clear opportunity to test whether an explicit, provenance-bearing spatial state and typed state transition solve a different bottleneck from a larger image-language backbone or more chain-of-thought prompting.

## 3. Testable architectural hypothesis

A state-transition candidate should:

1. retain competing admissible entity, geometry, frame and relation hypotheses where evidence is insufficient;
2. apply typed transformations only to the selected entity or world frame;
3. propagate the transformation through declared attachment/dependency closure;
4. recompute relations affected by the intervention;
5. preserve relations that are invariant under the transformation;
6. distinguish camera-, object- and world-centric relation semantics;
7. avoid treating derived views, shared source observations or duplicated projections as independent evidence;
8. revise only affected claims when a later observation contradicts the previous state, while retaining bitemporal lineage.

This is an experiment. No current Symthaea implementation or performance result is assumed.

## 4. Keep two lanes separate

### Lane A — known intervention

The candidate receives the pre-intervention evidence prefix, a typed transform and a query. The post-intervention scene is not shown. This isolates spatial-state representation, transform application and relation propagation.

### Lane B — observed intervention

The candidate receives before/after observations but not the evaluator's transform record. It must infer the intervention from the observations, align entity identities, update its state and answer. This includes visual grounding and correspondence.

Do not pool these lanes: Lane A tests transform-conditioned state update; Lane B tests whether the agent can infer the intervention and then update state.

## 5. Fixture matrix

`spacecast-intervention-matrix-v0.tsv` contains 12 development-only control/mutation pairs (24 cases). They are protocol probes, not benchmark samples or hidden holdouts.

| Pair family | Question |
|---|---|
| Local translation | Does the target relation change under the declared frame and transform direction? |
| Unaffected relation | Does the candidate preserve unrelated relations instead of rewriting the whole scene? |
| Local orbit | Are metric distances and orientation-dependent relations recomputed correctly? |
| Attachment closure | Do declared attachment dependents move with the parent, and only those dependents? |
| Occluder removal | Does visibility change without inventing removal of the target object? |
| Global yaw / world frame | Does world-centric direction update under a scene transform? |
| Global yaw / object frame | Are object-centric relations invariant to a common rigid transform? |
| Global yaw / camera frame | Is the fixed camera distinguished from the transformed world? |
| Bridge-view deletion | Does the candidate preserve uncertainty when the linking evidence is missing? |
| Observed intervention | Does the candidate infer a transform only from admissible before/after observations? |
| 3D provenance downgrade | Does stale or corrupted 3D metadata trigger a bounded 2D fallback or abstention? |
| Contradiction revision | Does new evidence retract only affected claims and preserve valid residuals? |

The matrix intentionally places expected behavior in a **development fixture**. It is not to be mounted in a sealed evaluation run as candidate input. Confirmatory truth and split custody remain with #6754.

## 6. Baselines and ablations

Use matched scenes, question IDs, initial evidence, transformation, model revision, decoding and resource budgets.

- Single-view general VLM.
- Multi-view general VLM with the same observation budget.
- Strongest reproducible SpaceCast reference model.
- Source-grounded 2D relation baseline.
- Candidate spatial state without intervention update.
- Candidate typed transform update.
- Candidate transform update with dependency closure and explicit invariant preservation.
- Oracle geometry / oracle transform, evaluator-only, as a diagnostic ceiling.

Factorial controls:
- same backbone, same evidence, with and without structured state;
- bridge views on/off;
- text/BEV 3D evidence on/off;
- known intervention versus observed intervention;
- dependency closure on/off;
- invariance ledger on/off;
- 2D fallback versus 3D claims gated by source/currentness;
- contradiction and correction on/off.

Never attribute extra input evidence, a larger model, changed prompts or extra compute to the state representation.

## 7. Scorecard

Keep each dimension independent:

- SpaceCast-native overall macro-average and L1/L2/L3 macro-averages;
- relation-change precision, recall and F1;
- preserved-relation accuracy and false-change rate;
- attachment dependency-closure exact-set accuracy;
- camera/object/world frame directional accuracy;
- object identity/cross-view correspondence;
- metric geometry error and unsupported-scale claim rate;
- claim revision latency and stale-claim persistence;
- 2D fallback regret under invalid or insufficient 3D evidence;
- selective risk/coverage and correct abstention by observability stratum;
- observations/bridge views consumed, tokens, compute, measured latency and energy/cost;
- attempted denominators, failures and NOT_EVALUATED reasons, clustered by scene.

Report paired scene-level differences and intervals. Do not use a universal weighted score to let one metric compensate for a wrong transform or false unique-map claim.

## 8. Prospective targets

These are research goals, not standards or achieved results:

- reproduce the native base-model comparator within ±2 absolute points on a rights-qualified, scene-disjoint subset;
- target ≥65% macro on a genuinely matched benchmark; stretch target ≥70%;
- keep L2 and L3 separately visible and never allow L1 gains to hide predictive regression;
- zero wrong-frame, wrong-direction, dependency-closure and false-invariance errors on deterministic controls;
- zero premature unique claims on ambiguous/unobservable controls, and revise after valid disambiguating evidence;
- at non-inferior task quality, target 20% fewer acquired observations than acquire-all on the controlled tournament;
- target ≥5 percentage points over the matched general baseline on preregistered held-out scene/layout strata, subject to confirmatory power analysis.

Compare the 65–70% goal to the paper's 58.0% base-model result only after matching model revision, prompt, evidence, split, task count and scorer. The 65.7% staged-SFT number is a separate trained comparator; reproduce its training protocol before using it as such.

## 9. Data rights and source limits

This first tranche carries only paper-level metadata and synthetic matrix descriptions; it downloads no ScanNet, ScanNet++ or SpaceCast assets. Before any data-backed run:
- inspect and archive the exact upstream dataset terms and each source asset licence;
- record the permitted purpose, redistribution restrictions and output policy;
- pin source commit, dataset revisions, transforms and scene split;
- keep scene IDs and all derived near-duplicate families on one side of the split;
- use #6754 for custody, candidate isolation and held-out evaluation.

SpaceCast is based on indoor ScanNet/ScanNet++ scenes and programmatic geometric transformations. Its reported labels do not establish real physical dynamics, contact forces, friction, collisions, deformation or manipulation safety.

## 10. Existing ownership boundaries

- #6183/#6184: AV scenario matrix and candidate projection contract.
- #7693: candidate projection validation and independent geometry/topology evaluation.
- #6675: active acquisition, deadlines and sequential policy tournament.
- #7577: DecisionNeed-relative sufficiency evaluator.
- #7884: failure-inclusive attempt accounting.
- #7902/#7910: observability-conditioned claim calibration.
- #6754: media rights, split validation, process isolation and blind custody.
- #7787: persistent structure versus transient occupancy.
- Symthaea VIS-012 #13855 / VIS-REP-001 #8152: candidate spatial-state and representation experiments.

This fixture package is an adapter and benchmark extension. It creates no new runtime engine.

## 11. Validation status

The CI audit checks JSON schema conformance and the TSV's row/pair structure only. A green run means these source artifacts are internally well-formed at that commit. It does not prove renderer correctness, transform code, Symthaea state construction, data rights, custody, performance, SOTA, physical truth or safety.
