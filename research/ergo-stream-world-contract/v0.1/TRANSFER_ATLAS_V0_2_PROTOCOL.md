# ErgoStream Transfer Atlas v0.2 protocol

**Status:** proposed benchmark protocol; not an executed confirmatory evaluation  
**Tracking:** [ERGO-STREAM-ATLAS-001 #7193](https://github.com/Luminous-Dynamics/symtropy/issues/7193)  
**Related:** #7149 hidden-world contract · #7171 transfer witness · draft PR #7173

## 1. Research claim and falsifiable scope

The Transfer Atlas tests a narrow, measurable claim:

> A learner can acquire a versioned structural library from causally arriving observations, transfer some of its structural knowledge to a previously unseen environment under an explicit transformation, identify when the mapping is underdetermined, and reject false analogies when structural or intervention evidence contradicts transfer.

This claim has four distinct levels. Never merge them:

1. **Representation correspondence:** the declared source and target structures preserve the specified finite relations under the proposed mapping.
2. **Predictive transfer:** the frozen map predicts held-out structural observations or continuations in the target.
3. **Intervention/effect transfer:** the frozen action/mechanism map predicts effects under controlled target interventions within a stated scope.
4. **Operational utility:** the acquired structure improves decisions or task outcomes over a matched direct policy and no-transfer control.

A result at level 1 is not evidence for levels 2–4. A sign-level reference test is not an authenticated intervention. Synthetic evaluation is not physical discovery.

## Existing owner composition

ErgoStream is a cross-repository research/qualification layer, not another learner, hidden-world runner, model DSL or sensor stack. The implementation should consume existing owners:

| Owner | Existing capability to reuse | ErgoStream-specific addition |
|---|---|---|
| Symthaea [#11042](https://github.com/Luminous-Dynamics/symthaea/issues/11042), [#11044](https://github.com/Luminous-Dynamics/symthaea/pull/11044) | Ansätz-free discovery contract; A0/A2/A3/A4/A5 split; structural identification, non-identifiability and positive/negative mechanism transfer | Freeze a versioned structural library and correspondence as explicit transfer artifacts |
| Symthaea [#11060](https://github.com/Luminous-Dynamics/symthaea/pull/11060) | Three executable finite world families, public `AgentView` / `AgentAction`, parent-runner/child-agent bridge, seven policy baselines, evidence-prefix digests and paired A0/A2 campaign | Add post-acquisition structural and finite-dynamics transfer outputs to this interface; do not create another JSONL episode runner or duplicate the existing seven acquisition policies |
| Symtropy [#5990](https://github.com/Luminous-Dynamics/symtropy/issues/5990) | Trusted owner-side interactive episode boundary and private world state | Add only typed stream-world adapters and hidden transfer probes through this custody boundary |
| Symtropy [#6027](https://github.com/Luminous-Dynamics/symtropy/issues/6027), [#6065](https://github.com/Luminous-Dynamics/symtropy/pull/6065) | Candidate-independent, bounded typed model DSL / interpreter and counterexample foundation | Encode or link candidate finite dynamics through the existing typed vocabulary; do not create a second scientific model AST |
| Symtropy [#7149](https://github.com/Luminous-Dynamics/symtropy/issues/7149) | ErgoStream stream-world provider and information-boundary contract | Supply paired stream inputs and withheld structural probes to the existing runner |
| Symthaea [#8919](https://github.com/Luminous-Dynamics/symthaea/issues/8919), [#8671](https://github.com/Luminous-Dynamics/symthaea/issues/8671), [#8705](https://github.com/Luminous-Dynamics/symthaea/issues/8705) | Shared grammar recovery, progressive knowledge acquisition / negative-transfer concerns, invariant mechanism transfer | Treat learned grammar and mechanism fragments as candidate structural libraries with explicit transfer scope |
| Symthaea [#8669](https://github.com/Luminous-Dynamics/symthaea/issues/8669) | Equivalence classes, posterior uncertainty, support omission and abstention | Preserve set-valued or unresolved correspondences when a unique map is not identifiable |
| Symthaea [#12837](https://github.com/Luminous-Dynamics/symthaea/issues/12837) | AV-STREAM timing, multimodal continuity and active sensing | Reuse as the audiovisual acquisition adapter, not a new clock/fusion layer |
| Symthaea [#9094](https://github.com/Luminous-Dynamics/symthaea/issues/9094), [#13389](https://github.com/Luminous-Dynamics/symthaea/issues/13389) | Decision-centric world-model comparison and RoboDyna / LIBERO-Dynamic adapters | Measure whether transferred structure improves action outcomes under matched budgets |
| Symthaea [#13375](https://github.com/Luminous-Dynamics/symthaea/issues/13375), [#13377](https://github.com/Luminous-Dynamics/symthaea/pull/13377) | Causal trajectory pairing, evidence filtration and action/outcome provenance | Reuse the existing trajectory identities for prediction-to-action qualification |

The tiny fixtures in Symtropy #7173 are contract-conformance tests only. The confirmatory world corpus should be layered onto these owners rather than implemented as a second simulator or process protocol.

### Transfer-map semantics: from isomorphism to scoped morphisms

The v0.1 validator deliberately accepts only total, one-to-one correspondences over two complete finite structures. Keep this strict contract as the integrity floor.

The first scope-aware implementation is now in `src/scoped.rs` and supports three finite map kinds:
- **Isomorphism:** complete source and target structures.
- **Induced embedding:** a selected source and target region, with all relations wholly inside each selected node set included.
- **Subgraph embedding:** selected source and target subgraphs; additional target relations are allowed only when explicitly declared outside the target scope.

Each side declares selected and excluded node/relation IDs. They must be disjoint, valid, unique and together partition the complete structure. The candidate mapping must be total over the selected substructures. The output reports the excluded context counts.

The v0.2 schema `schemas/scoped_transfer_proposal_v0.schema.json` carries source/target episode IDs and evidence-prefix digests. It is a separate proposal sidecar, not an incompatible mutation of the existing #11060 AgentEpisode v0 schema.

Other map kinds remain future work:

| Map kind | Meaning | Required validator |
|---|---|---|
| Homomorphism | Relation-preserving many-to-one mapping | Explicitly declared collapsed distinctions and relation preservation |
| Quotient / coarse-graining | Partition fine states/entities into abstract classes | Equivalence partition and induced abstract relations/observables |
| Simulation | Every in-scope source transition is matched by a target transition | One-way action/state mapped transition coverage |
| Bisimulation | Both systems mutually simulate each other | Forward and backward transition coverage |
| Approximate simulation / bisimulation | Dynamics correspond within declared error bounds | Norm, tolerance, horizon, support and uncertainty fixed before holdout |
| Conjugacy / semiconjugacy | Continuous dynamics commute under state/action/time maps | Clock map, flow support, hybrid event/reset semantics and trajectory-level validation |

The evaluator must score the declared map kind, not silently upgrade an embedding to an isomorphism or a one-way simulation to a bisimulation. This implementation is still a finite structural validator; it does not qualify mechanism or dynamics transfer by itself.

## 2. Evaluation custody and episode contract

### Owner / evaluator process

The trusted owner process holds:
- generator family, parameters and private seed;
- latent graph / hypergraph and true relation identities;
- intended transformation and whether the pair is a false analogy;
- holdout streams, probes and intervention outcomes;
- scoring implementation and replay material.

### Learner process

The learner receives only:
- validated, schema-versioned causal observation frames;
- public timing, clock uncertainty, modality, channel and action semantics;
- observations already acquired and loss / discontinuity markers;
- permitted queries or actions and their declared cost;
- opaque episode and candidate-local identifiers.

The learner must not receive:
- source code/configuration for the generator;
- private seeds or deterministic seed derivation;
- future frames, withheld target probes or hidden relation IDs;
- evaluator labels, pair-family names or expected dispositions;
- intervention outcomes before the transfer / action map is frozen.

The owner must serialize an allowlisted policy view and reject unknown fields. Do not pass references to evaluator objects into the learner. Process separation or a Rust module boundary alone is not a security boundary. A sealed evaluation claim requires a qualified OS/container boundary, restricted filesystem and network access, independent seed custody, bounded resources and auditable process receipts.

### Freeze and reveal order

1. Owner samples source / target world pair and records private replay recipe.
2. Learner observes the permitted acquisition stream and may spend its acquisition budget.
3. Learner emits a complete, versioned structural-library snapshot.
4. Learner emits its source-target correspondence, scope, assumptions, alternatives and action-kind/effect transforms.
5. Owner binds these artifacts to the exact learner/configuration identity and freezes them.
6. Owner releases hidden structural holdouts and, separately, hidden intervention outcomes.
7. The evaluator scores without permitting the learner to rewrite the frozen artifact used for the primary result.
8. Any later adaptation is a new versioned episode, not a retroactive revision of the original score.

## 3. Paired-world generator families

Begin with finite deterministic generators and independent exact oracles. Introduce families incrementally; do not make the first confirmatory claim on two graph fixtures.

| Family | Positive / target condition | Required negative control |
|---|---|---|
| Isomorphic relabeling | Same structure under symbol, entity, node, relation and channel relabeling | Many-to-one or inconsistent relabeling |
| Equivariance | Declared time dilation, direction reversal, sign reversal or coordinate transform | Unannounced / inconsistent transform |
| Degree- and marginal-matched | Same node degree and local marginals; higher-order structure differs | K3,3 / triangular-prism style global-topology difference |
| Compositional transfer | Recombine acquired relation primitives into unseen motifs and higher-order structures | Familiar primitives used in an incompatible composition |
| Mechanism collision | Superficial patterns match but functional / causal mechanism differs | Observationally similar pair with divergent intervention outcome |
| Non-identifiability | Available prefix admits multiple mappings | Evaluator reveals a permitted discriminating query; otherwise accept set-valued or unresolved response |
| Multiscale structure | Relations survive, transform or disappear under explicit coarse-graining | False preservation after information-losing abstraction |
| Cross-modal alignment | Coupled event streams with delay, drift and missing channels | Incorrect association, temporal shuffling and missing-modality controls |
| Interactive dynamics | Occlusion, contact, event order and continuous-time state changes | Same snapshots with different causal ordering / action outcomes |

Continuous-time and hybrid dynamics qualification is specified separately in [`CONTINUOUS_DYNAMICS_TRANSFER_V0_2_PROTOCOL.md`](CONTINUOUS_DYNAMICS_TRANSFER_V0_2_PROTOCOL.md). It defines explicit clock/time maps, irregular sampling, delay, hybrid events/resets, uncertainty decomposition and trajectory-level transfer error. This is a protocol only; the executable v0.1 witness remains finite and deterministic.

The current v0.1 Transfer Atlas fixtures cover a relabeled/reversed positive pair, a degree-matched non-isomorphic pair, an isomorphic observed structure with hidden mechanism non-transferability, and a qualitative intervention-divergent effect fixture. Separately, #7173 now contains a deterministic finite-state dynamics witness that checks exact transition commutation under a proposed state/action map. Neither the fixtures nor the validator implement these full generator families, stream-to-graph learning, an authenticated intervention runner, or a sealed evaluation service.

## 4. Baseline ladder and comparison discipline

### Acquisition controls

- no acquisition / no-memory control;
- passive fixed-rate exposure;
- random observation / query policy;
- prediction-error / surprise-only;
- learning-progress;
- information gain and cost-normalized information gain;
- predictive compression / MDL;
- strong generic event segmentation and statistical motif controls.

### Structural correspondence controls

- unique degree / incidence-signature baseline;
- exact graph-isomorphism (VF2-style, Bliss, Nauty/Traces or equivalent, task permitting);
- graph-edit / assignment / subgraph matching (NetworkX GED or stronger optimized implementation where applicable);
- learned graph embeddings / GNN / graph transformer;
- dense embedding retrieval or nearest-neighbour analogy;
- Symthaea HDC-only, CfC/LTC-only, explicit-graph-only, symbolic/e-graph-only and hybrid ablations.

### Causal / interactive controls

- ACDB-compatible PC plus greedy active orientation and density-aware structure-blind controls;
- Unsatz-compatible model-form / experiment-selection baselines on its native protocol;
- direct reactive / VLA policy without an explicit world model;
- action-conditioned world model + planner;
- acquired-structure policy and fresh-learning-from-scratch target control;
- human sample / action-efficiency reference when meaningful.

Do not compare against a weak random graph baseline without matching degree, density and size. Use two separate comparison tracks:

1. **Matched budget:** identical observations, interventions, model calls / compute, memory, wall time and action budget.
2. **Best available:** strongest admitted public or commercial system under its native supported setup, with all differences explicitly documented.

External benchmark leaderboard values are context until the exact protocol is run. Do not claim head-to-head superiority by comparing results from different datasets or input conditions.

## 5. Scorecard

Report per family, transformation class, size regime and budget. Do not collapse to a single ErgoScore.

### Capability

- node / relation correspondence precision, recall and exact-match rate;
- invariant, equivariant and non-transfer classification accuracy;
- held-out continuation and intervention-effect prediction;
- unseen composition success;
- calibration of relation and map scope;
- downstream action success, regret, partial progress, recovery and constraint violations.

### Proposal / validator / mechanism outcomes

Record transfer disposition as a three-stage outcome rather than collapsing every rejected map into abstention:

- **Abstention:** the learner emits no transfer proposal. Score correct abstention on non-identifiable or evaluator-negative pairs separately from missed transfer on positive pairs.
- **Rejected proposal:** the learner emits a candidate map, but the structural or dynamics witness rejects it. Record the specific counterexample and rejection reason; do not call this abstention.
- **Accepted correspondence:** the candidate map passes the declared structural / dynamical validator. Then compare it against the evaluator-owned mechanism-transfer truth and held-out interventions. A valid structural map can still be a **false unification** if the underlying mechanism does not transfer.

Report proposal coverage, proposal rejection rate, accepted-structure precision, mechanism false-unification rate, correct-abstention rate and missed-transfer rate separately. This avoids rewarding an invalid proposal as if the learner had abstained, and avoids treating a graph isomorphism as evidence that the mechanism is shared.

### Epistemic reliability

- **false-unification rate:** structurally accepted correspondence on evaluator-defined non-transferable mechanism pairs; keep this separate from structural-validator rejection rate;
- **false-invariance / false-equivariance rate:** unsupported transform claim;
- **false-unique rate:** one asserted map where the available evidence is non-identifying;
- **false-causal-transfer rate:** effect / mechanism transfer asserted but contradicted by valid intervention evidence;
- **proposal rejection rate:** candidate maps rejected for structural / dynamical counterexamples (not the same as abstention);
- risk–coverage, selective accuracy and abstention quality;
- negative-transfer cost and counterexample correction / retraction latency.

### Efficiency

- observations and active queries to first validated structure / transfer;
- interventions required to discriminate candidate mechanisms;
- CPU/GPU time, training and inference compute, model/API calls, tokens;
- wall time, peak memory and energy where measured;
- total cost per validated transfer and per successful target action.

Publish Pareto frontiers for transfer vs false-analogy risk, transfer vs all-in acquisition cost, and target task utility vs latency / resource cost. Include worst-family performance, not only pooled averages.

## 6. Prediction-to-action advantage test

A learned representation is valuable only if it changes a downstream decision in a beneficial way.

On every admitted interactive domain, compare:
- direct reactive policy with no explicit learned world model;
- action-conditioned world model plus planner;
- structural-library transfer plus the same planner / action interface;
- fresh target-domain learning without source structural memory;
- oracle dynamics only as an evaluator upper bound.

Keep sensor input, action API, policy class where possible, horizon, replanning frequency, wall-time and compute budget matched. Report:
- success / partial progress and recovery;
- constraint / safety violations;
- action regret against an admitted oracle or reference policy;
- sensitivity to inference latency and stale observations;
- additional observations / interventions consumed;
- incremental performance from the acquired structure over the direct-policy baseline.

A visual prediction or graph score is not a substitute for measured action utility.

## 7. Statistical and reproducibility requirements

- Split by complete generator family, mechanism, composition and transformation—not merely random frames from the same world.
- Keep the final confirmatory family / seed set evaluator-owned.
- Use pilot runs to estimate between-world and between-family variance; perform power analysis before freezing the confirmatory sample size.
- Treat independent world pairs / episodes as sampling units; frames within an episode are not independent replicates.
- Pre-register primary metrics, guardrails, exclusions, missing-data handling and non-inferiority margins before unblinding.
- Report paired uncertainty intervals and per-family outcomes. Use hierarchical analysis where family-level heterogeneity matters.
- Preserve every failed, invalid, timed-out and incomplete run in the denominator and cost record.
- Pin source commit, model / weight hash or API revision, prompts, preprocessing, action semantics, tool access, environment config, dependencies and hardware.
- Publish replay inputs, evaluator version, output digests and counterexamples subject to licensing and safety constraints.

## 8. Proposed go / no-go gates

These are prospective internal research targets, not standards or achieved results:

- 100% exact agreement with independently derived finite oracles on in-scope finite cases;
- zero learner-visible seed, future-frame, hidden-label or withheld-probe leakage in serialized policy views;
- zero confident causal promotion from observational-only controls;
- no forced unique answer on evaluator-defined non-identifiable cases;
- at least six structurally distinct generator families before a confirmatory cross-family transfer claim;
- at least +10 percentage points in primary transfer success over the strongest matched baseline, **or** at least 20% lower all-in observation / compute cost at a preregistered non-inferiority margin no wider than 2 percentage points.

Estimate pilot variance and statistical power before confirming the thresholds. False-unification and false-causal-transfer guardrails are non-compensable: better average accuracy cannot offset unsafe false structure.

## 9. Falsification conditions

Narrow or reject the thesis if:
- simple statistics or exact graph matching achieve the same transfer result at lower total cost;
- HDC / CfC / graph components add no measurable improvement after compute and data are controlled;
- gains disappear under whole-family or unseen-composition holdouts;
- the learner confidently unifies degree-matched or intervention-divergent negative pairs;
- structural correspondence repeatedly fails to predict downstream effects;
- direct reactive policies match or exceed the acquired-structure policy at lower cost;
- the learner cannot represent unresolved maps and scope boundaries.

A null result is informative: it shows when structural memory is not worth acquiring.

## 10. Claim boundary

A passing finite reference test establishes only the declared finite property of the fixture and validator. A valid structural mapping does not establish semantic understanding, causation, mechanism transportability, biological equivalence, consciousness, universal learning, physical discovery or SOTA performance. Those claims require separate, independent evidence at the appropriate tier.
