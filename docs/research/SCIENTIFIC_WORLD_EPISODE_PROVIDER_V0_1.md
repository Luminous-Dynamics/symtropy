# Scientific World Episode Provider v0.1

**Status:** research contract / documentation-only draft  
**Issue:** [SCIENTIFIC-WORLD-EPISODE-001](https://github.com/Luminous-Dynamics/symtropy/issues/5957)  
**Consumer:** [Symthaea #11042](https://github.com/Luminous-Dynamics/symthaea/issues/11042) and [#11171](https://github.com/Luminous-Dynamics/symthaea/issues/11171)  
**Parent baseline:** Symtropy `main` at `2a3152c03e2f7ec3b6f430ebaa1a0a115ce4e2f4`

This document freezes the first policy-visible boundary only. It does not implement a world generator, simulator, agent, scheduler, evaluator, cryptographic receipt, or live laboratory interface.

## 1. Research objective

Expose existing Symtropy-owned executable models as controlled scientific environments in which a blind research agent can:

1. form competing structural/mechanistic hypotheses;
2. choose observations and interventions under declared costs;
3. predict outcomes before observing them;
4. distinguish model fit from structural identification;
5. preserve non-identifiability and abstain where evidence is insufficient;
6. predict held-out interventions and longer horizons; and
7. transfer or explicitly reject mechanisms in related worlds.

The unit of evaluation is not a fluent research report or a visually plausible world. It is a **prospective, evidence-prefix-bound research decision followed by an independently evaluated outcome**.

## 2. Owner boundaries

| Concern | Owner | This adapter must not |
|---|---|---|
| Domain dynamics, units, model applicability, observation/intervention semantics, deterministic replay | Symtropy domain/model owners | Add a parallel physics engine, generic causal model, or alternate source of physical truth |
| Research policy, hypotheses, experiment choice, predictions, interpretation, abstention | Symthaea | Give the policy evaluator truth, hidden seeds, answer keys, or scoring hooks |
| Challenge construction, sealed truth, task validity, held-out splits and independent scoring | Symthaea #9332 / independent verifier | Create a second oracle or let the world provider score its own task |
| Existing receipts, evidence lineage, source identity and invalidation | Existing Mycelix / Symtropy authorities | Add a second evidence ledger, trust graph, or signer |
| Host isolation and workload lifecycle | Luminous Platform / Nixward owners | Add host authority or production control paths here |

The provider is a thin adapter across these owners. A provider record is not an authority grant.

## 3. Trust and information-flow model

Assume the research policy may be mistaken, reward-seeking, compromised, or deliberately adversarial. Assume all public source code, schemas, public fixtures and prior public runs are visible to it.

The policy-visible process may receive only:
- the research objective and task constraints;
- declared units and observation semantics;
- the available instrument/intervention catalogue and costs;
- the current observations that have actually become available;
- the declared remaining budget and decision deadline;
- the evidence class and claim ceiling; and
- opaque episode identifiers and digests needed to bind the visible record.

It must not receive:
- latent state or generating equations unless the task explicitly tests the supplied-structure A0 control;
- hidden parameters, private seeds, future random draws or future trajectories;
- sealed calibration/confirmation labels or held-out outcomes;
- evaluator-only scores, split labels, oracle handles or filesystem paths;
- unobserved branch outcomes; or
- an action result before the provider has executed and returned it.

The policy and evaluator must be separate processes or services for any blind claim. A Rust type boundary or a shared in-process object is not a security boundary. OS sandboxing, filesystem/network isolation, resource limits and secret custody are external prerequisites and must be qualified separately.

## 4. V0.1 public episode packet

The machine-readable contract is `public-episode-packet.schema.json`; the example is a development fixture, not a blind evaluation task.

The packet contains:
- schema ID and version;
- an opaque episode instance ID and an episode-contract digest;
- task objective and declared scope;
- observation channels with value type, units, quality states and time semantics;
- legal instruments/interventions with typed parameter bounds, costs and expected durations;
- explicit budget dimensions;
- clock and time-origin identities;
- initial observations with event/acquisition time and availability time; and
- evidence class and a claim ceiling.

The packet deliberately contains no `seed`, `split`, `answer`, `ground_truth`, `oracle`, `score`, or hidden model parameters field. If `owner_profile_ref` is present, it may resolve only to a public model/profile definition—not a per-episode state, private seed, answer key, evaluator location or oracle handle. The schema rejects unknown top-level fields. Domain-specific payloads must be added through a versioned extension that is reviewed for leakage, not smuggled through arbitrary metadata.

A public episode packet does not define Symthaea's `AgentView` or `AgentAction`. The consumer adapter in Symthaea #11171 remains responsible for mapping the packet and subsequent observations into the existing blind-episode protocol.

## 5. Time, evidence and action semantics

Keep these instants distinct:
- **event time:** when the modeled phenomenon occurred;
- **acquisition time:** when an instrument sampled it, if applicable;
- **availability time:** when the observation became available to the policy;
- **decision time:** when the policy selected an action; and
- **execution time:** when the provider applied or completed the action.

An observation that arrives after a decision cannot be used to justify that decision during replay. Late evidence may be used in a later decision, never retroactively.

An action has a requested disposition and a provider-observed disposition. At minimum preserve `requested`, `accepted`, `rejected`, `applied`, `failed`, `timed_out`, and `unknown` as distinguishable states. The provider must not silently convert an unknown outcome into a successful retry. Every request, including malformed, prohibited, failed and incomplete requests, remains in the campaign denominator.

Evidence classes are ordered by claim meaning, not numerical confidence:
- `synthetic`: generated from a declared synthetic process;
- `modelled`: produced by a declared executable model within its stated applicability domain;
- `measured`: recorded at an identified measurement boundary; and
- `independently_qualified`: accepted under a named external qualification process.

A modelled plant does not become measured because a real controller or an agent interacted with it. Mixed controller/plant evidence must preserve both classes and use the stricter claim ceiling.

## 6. Counterfactual branch contract

A comparison between candidate actions must start from the same declared decision boundary. Bind, at minimum:
- exact world/model/profile and initial-state identities;
- exact observation history and evidence-prefix digest;
- agent policy/checkpoint and recurrent-state identity;
- candidate-action set and declared resource budget;
- simulator and actor-policy revisions; and
- randomness protocol and exogenous-stream identity where justified.

Recompute any actor response whose inputs change under intervention. Do not reuse stale actor actions. Common random numbers may be paired only for disturbances independently declared invariant to the intervention. Otherwise branch randomness must be resampled and the difference reported.

A branch is invalid when its initial state, information set, actor policy, action catalogue, budget or relevant model identity differs without a preregistered reason. A branch audit is not a physical-world causal identification claim.

## 7. World and split design

Reuse the existing world/genesis, model, replay and benchmark authorities. Do not introduce a separate world namespace or a generic scientific DSL.

Initial development families should cover:
1. hidden-state / lagged dynamics;
2. additive versus interacting inputs, including collinearity and non-identifiability;
3. measurement ambiguity: distortion, missingness, saturation and uninformative channels; and
4. out-of-support interventions, parameter ranges, prediction horizons and held-out mechanism families.

Split by **mechanism, recipe, measurement family, topology and domain**, not only by random episode rows. Keep development, calibration, confirmation and rotating challenge partitions distinct. The sealed evaluator owns seeds and answer keys. A generator may propose candidate tasks; a separate validity gate must check construct preservation, determinacy, resource bounds and oracle correctness before the task enters a qualification set.

Generated worlds are experiment material. They are not evidence that the modeled process exists in nature, and generator diversity is not itself scientific validity.

## 8. Required comparison arms

Under the same policy-visible starting evidence and inclusive budget, compare:
- no-model / no-optional-probe;
- fixed-grid and matched-cost random probes;
- supplied-structure parameter fitting (A0);
- family discovery and latent/measurement-structure discovery (A1–A3);
- cost-normalized information gain and decision-aware EVSI where their assumptions are justified;
- a small exact oracle as a diagnostic upper bound only;
- frozen Symthaea incumbent and versioned challenger; and
- human/domain-expert research decisions with pre-run forecasts and intervention time retained.

Do not confound orchestration with extra tools, stronger model backends, extra context, or larger experiment budgets. Report total model/tool tokens, CPU/GPU time, memory, simulation steps, wall time, energy and human review separately.

## 9. Metric vector (no opaque composite)

Report independently:
- variable/latent-state/measurement-map precision and recall;
- structural-family/equivalence-class recovery and false-unique rate;
- mechanism recovery only where the permitted interventions identify it;
- predictive log score or CRPS, calibration, held-out intervention error and full horizon-error curve;
- action-ranking regret, counterfactual prediction error and closed-loop utility;
- correct and unnecessary abstention;
- experiment count, realized information gain, net decision value and opportunity cost;
- correct continue / pivot / stop / replicate / escalate decisions;
- cross-world transfer, negative transfer and worst-family regression;
- p50/p95/p99 latency, wall time, CPU/GPU, memory, tokens, energy and human time; and
- trace completeness, denominator conservation, leakage failures and claim/evidence integrity.

Do not average a hard integrity failure, false-unique mechanism claim, or critical applicability violation away with a strong prediction score.

## 10. Prospective qualification targets

These are proposed targets, not measured results:
- zero policy-visible hidden-truth, seed, future-outcome or split-label leakage in the adversarial access corpus;
- 100% action, branch and attempt denominator reconciliation;
- zero false-positive unique-mechanism claims on exhaustive synthetic non-identifiability and open-support controls;
- 100% rejection of stale branch state and unjustified common-random-number pairing in the authored conformance corpus;
- at least five independent seeds and multiple held-out structural/measurement families for confirmatory comparison;
- positive paired improvement over the strongest compatible simple baseline in action regret or fully inclusive cost per independently verified mechanism, with uncertainty intervals and no protected integrity, calibration, worst-family, latency or energy regression; and
- positive transfer to at least one genuinely held-out mechanism family and one fresh scientific domain before any broad research-autonomy claim.

Choose numerical superiority margins and sample size after a debugging pilot and prospective precision analysis. Do not borrow a percentage from an unrelated benchmark.

## 11. External comparison discipline

Relevant, non-interchangeable references:
- **Unsatz** (2026): structural discovery without a supplied ansatz. Its paper reports 80 systems across 20 domains and 21–60% solved across nine models. It is the closest construct match; direct claims require its exact track and version.
- **RE-Bench** (ICML 2025): seven research-engineering tasks with 71 eight-hour human attempts; results vary with time budget, so capability must be plotted against total time and resources.
- **ASI-Bench** (2026): methodological autonomy under progressively withdrawn guidance; it is not a scientific-validity or mechanism-recovery score.
- **Autonomous Science Bench** (2026): six laboratory workflows with distinct discovery and held-out-learning gates; simulated outcomes must not be represented as physical experiments.
- **FIRE-Bench / PaperBench / SciAgentArena:** complementary full-cycle rediscovery, paper reproduction and stepwise research-workflow evaluations.
- **World Model Benchmark Survey** (2026): its 130-benchmark snapshot shows a gap between open-loop/prediction-level evaluation and closed-loop/downstream-outcome evaluation. This supports decision-centric measures, not a blanket claim that the present project is superior.

Do not compare raw scores across different task sets, splits, rubrics, runners, time budgets or model versions. Publish a parity matrix before any leaderboard claim.

## 12. Existing Symtropy discovery evidence and the next bottleneck

The provider must extend—not relabel—the current SIM-REL results. The following are reported in open PRs and have bounded claim ceilings:

| Existing work | Reported evidence | What it does not establish |
|---|---|---|
| [SIM-REL-002 PR #2904](https://github.com/Luminous-Dynamics/symtropy/pull/2904) | 18 hand-designed Boolean pilot cases; mean intervention count 7.833 for maximum-disagreement, 7.778 for fixed ascending, and 7.748 across seeded-random repetitions. | The active max-disagreement selector did not beat these simple baselines on this pilot; it is not physical-system discovery or cross-domain transfer. |
| [SIM-REL-003 PR #2916](https://github.com/Luminous-Dynamics/symtropy/pull/2916) | Exhaustive census of all 256 three-input Boolean functions. MDL-EIG/cost beta 1 reduced mean cost from 21.648 to 21.355 (about 1.35%), but out-of-grammar detection fell from 81.6% to 74.8%; held-out OOD detection fell from 73.3% to 66.7%. | A small finite-domain efficiency/robustness trade-off, not a general scientific-research win or physical-world result. |
| [SIM-REL-014 #3021](https://github.com/Luminous-Dynamics/symtropy/issues/3021) | Already specifies stateful stochastic experiment design with physical/sensor persistence, confounding controls, risk-cost-abstention frontiers, exact tiny-world oracles and matched baselines. | Its broader target suite is not a reason to create a second optimizer or oracle here. Consume it where the model and estimand match. |

These results identify the highest-value next bottleneck: **a method that lowers cost without purchasing that gain through worse misspecification detection, false certainty, or transfer failure**.

Therefore:
- retain misspecification detection, false-unique claims, and calibration as hard, separately reported outcomes;
- compare selectors on a cost–risk–abstention–transfer Pareto frontier, not mean intervention count alone;
- include adversarial but scientifically valid worlds whose mechanisms, sensor maps, delays or topology differ from development;
- test whether a mechanism transfers, partially transfers, or must be rejected; and
- choose the first Symtropy model adapter based on an existing model's explicit domain and observable intervention semantics, not on ease of generating a visually rich world.

The open PRs are not merged mainline results. Their stated exact-head checks belong to those PRs and do not qualify this provider branch.

## 13. Implementation sequence

1. **Contract:** freeze public packet, world profile and trace schemas; add development known-answer examples.
2. **Leakage/conformance:** validate closed schemas, opaque IDs, observation availability, legal intervention/costs, branch identity and denominator conservation.
3. **Owner adapter:** select one existing Symtropy model with an explicit applicability domain; implement a thin adapter and exact replay test.
4. **Symthaea consumer:** use #11171 to connect the actual research policy; retain #11060 as a protocol reference, not as a learned researcher.
5. **Pilot:** run matched baselines and debugging seeds; inspect variance, failures and resource accounting.
6. **Confirmation:** preregister a held-out family, total budgets, primary metrics and stopping rules; use the existing sealed evaluator and statistical owners.
7. **Transfer:** test fresh mechanisms/domains; promote claims only under the existing Symthaea #10497 process where relevant.

## 14. Claim ceiling

This v0.1 document and schema establish only a proposed policy-visible interchange contract. They do not implement or qualify a world provider, demonstrate agent performance, validate a physical model, prove a scientific discovery, establish autonomous laboratory operation, or support SOTA / superintelligence / production-safety claims.
