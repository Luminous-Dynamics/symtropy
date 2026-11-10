# LOCO-RECOVERY-TOPOLOGY-001
## Fault-domain-independent recovery pathways and topological mobility reserve

**Status:** protocol / finite known-answer corpus, v0.1  
**Canonical issue:** https://github.com/Luminous-Dynamics/symtropy/issues/4511  
**Canonical fault campaign:** https://github.com/Luminous-Dynamics/symtropy/issues/4358

## 1. Research question

How many genuinely independent, robustly complete routes to a declared recovery state remain after an actuator, contact, sensing, timing, power or common-cause fault—and what smallest set of shared physical failure domains can destroy every route?

This work treats recoverable mobility as a property of the topology of the robot's **robust continuation graph**, not the number of actions shown in a controller's menu.

A controller may expose many actions or gait labels that all depend on the same actuator, stance leg, power bus, timing source or fragile intermediate state. Raw action count can therefore suggest abundant options where the physical system has only one failure path.

The proposed output is a **topological mobility-reserve vector**: robust route existence, physically distinct recovery regions, failure-domain-independent pathways, minimal cut sets, worst-case horizon, recovery energy and limiting resource margins. It is not a weighted score.

## 2. Explicit ownership boundaries

This is a composition and measurement layer. It must consume, not replace:

| Existing authority | What this work consumes |
|---|---|
| #4061 | Finite nondeterministic robust viability, recovery reachability, horizons and admitted-action witnesses |
| #4083 | Sound set-valued hybrid transition abstraction |
| #4130 / #4133 | Axiswise tested uncertainty-reserve brackets |
| #4304 / #4308 | Coupled uncertainty frontiers |
| #4119 | State-, model-, sensor-, actuator- and time-dependent evidence validity |
| #4342 / #4345 | Source-qualified common-cause fault signatures and failure-domain identities |
| #4332 / #4334 | Health-conditioned failover and viability-gated mode transitions |
| #4310 / #4313 / #4293 | Viability-conditioned diagnostic action and post-probe continuation evidence |
| #4324 / #4326 | Complete adverse-event taxonomy and category-wise comparison |
| #4327 / #4329 | Held-out empirical capability and tail-performance evaluation |
| #4338 / #4340 | Evidence-qualified operating-envelope conjunction |
| #4358 / #4359 / #4493 | Canonical matched actuator-fault campaign, protocol and auditor |
| #4438 / #4441 | Graceful-degradation and robust-recovery experiment design |
| #4360 / #4362 | Morphology, structural redundancy and fault-aware embodiment co-design |

The new auditor, if implemented, owns only:
- canonical physical-path equivalence;
- failure-domain dependency composition;
- bounded resource-group cut-set and path-independence calculations;
- output witnesses and evidence-integrity checks for this derived reserve vector.

It does **not** calculate viability, generate physical transitions, choose controls, estimate fault probabilities, infer complete failure-domain coverage or certify physical safety.

## 3. Definitions

### 3.1 Robust continuation graph

Construct a bounded, time-expanded graph from the upstream transition abstraction. A node is an exact state/context at a declared horizon index. An edge is an action transition with its full physically possible successor set, source identity and resource dependencies.

An action transition is usable only if the upstream authority admits it for every physically possible outcome under the declared fault/uncertainty set. A probability of zero does not remove a physically possible branch from robust admission.

Every route must:
1. Start at the exact evaluated state and context.
2. Use only upstream-admitted actions.
3. include every intermediate state and every physically possible branch.
4. remain inside the supplied viability set throughout execution and observation/actuation latency.
5. satisfy the declared horizon, energy, actuator, thermal, contact and timing bounds.
6. reach a declared recovery target or explicitly typed safe terminal state.
7. bind every step to fresh, identity-matched upstream receipts.

A route that is terminally viable but passes through a nonviable intermediate state is not a qualifying route.

### 3.2 Physical-path equivalence

Different action IDs, policy labels, gait names or receipt IDs do not establish different physical routes.

The canonical path identity must be derived from the ordered, time-indexed physical transition identities and declared terminal/recovery-region identity. Alias actions that generate the same physical path are one route class. Equivalent paths with only record-order or identifier changes must produce the same canonical result.

Where the upstream abstraction cannot establish physical equivalence, return Unknown rather than assume independence.

### 3.3 Failure domains

Each route carries a complete, source-bound set of failure-domain dependencies. Candidate domain families include:
- joint, actuator, drive and transmission;
- leg, support and contact chain;
- power bus, battery, regulator and thermal-management chain;
- position/velocity encoder, IMU, contact sensing and shared fusion source;
- clock, synchronization, communications and compute;
- structural members, couplings, brakes and other mechanical paths.

A dependency is a physical or common-cause resource whose loss can invalidate the route. The domain manifest must preserve shared-source and common-cause grouping from the upstream fault model; it must not infer independence from separate sensor names or controller modules.

A route using two separate legs but one common power bus has two mechanical branches and one shared power dependency. The shared dependency remains visible and can form a one-element cut set.

### 3.4 Independence and cut sets

For the frozen v0.1 contract:
- a set of routes is failure-domain-disjoint only when their declared dependency sets are pairwise disjoint;
- the maximum number of pairwise domain-disjoint routes is the independent-route count;
- a cut set is a set of declared failure domains whose removal intersects every qualifying route;
- the minimum cut-set size is the smallest such set; enumerate every minimal cut set within the declared resource bound.

The report must show both route-level dependency sets and the cut-set witnesses. A minimum cut of size one exposes a single declared domain whose failure destroys all qualifying recovery routes.

This is a finite combinatorial property of the declared graph and domain manifest. It is not a claim that the manifest contains every real physical common cause.

### 3.5 Distinct recovery regions

Count distinct endpoints only after applying a declared equivalence relation over physical recovery states. Distinct support modes, body postures, foothold sets, actuator configurations and safe terminal states may be reported separately when their physical distinction is evidenced.

Do not count a renamed state, duplicated node, policy label or different action sequence that converges to the same physical state as a new recovery region.

## 4. Output: topological mobility-reserve vector

Report the following independently:

| Component | Meaning |
|---|---|
| Robust reachability | Whether a complete robust route exists to each declared recovery target |
| Route classes | Number of unique physical route classes after alias/equivalence handling |
| Recovery-region diversity | Number and identities of distinct evidenced terminal support/recovery regions |
| Independent-route count | Maximum pairwise failure-domain-disjoint qualifying routes |
| Cut-set resilience | Minimum failure-domain cut-set size plus every minimal cut witness |
| Horizon reserve | Worst-case remaining steps/time to recovery and limiting intermediate state |
| Energy reserve | Worst-case and per-route energy to recovery, measurement boundary and remaining budget |
| Physical margins | Supplied actuator, thermal, support, contact and timing margins |
| Fault coverage | Exact fault × terrain × payload × morphology cells with qualifying recovery routes |
| Evidence completeness | Freshness, identity joins, branch coverage, graph bounds and Unknown/Invalid reasons |

No single total or weighted “resilience score” is emitted. A user may not trade a fall, missing failure domain or nonviable branch against a high action count.

The result is not a continuous volume and must not interpolate across untested graph states, fault severities or terrain cells.

## 5. Baseline comparisons

Run all baselines on the same frozen graph and context:

1. Raw admitted-action count (intentionally weak).
2. Distinct immediate successor count.
3. Distinct terminal/recovery-region count.
4. Ordinary edge-disjoint route count.
5. Ordinary vertex-disjoint route count.
6. Failure-domain-aware pairwise route disjointness and group cut sets.
7. Upstream #4061 reachability and worst-case recovery horizon without topology analysis.

The main hypothesis is that action and successor counts overstate recoverable reserve when options share a hidden physical dependency. The second hypothesis is that failure-domain-aware cut sets predict held-out recovery outcomes better than raw option counts. Both are falsifiable empirical hypotheses, not safety guarantees.

## 6. Known-answer and adversarial fixture plan

The frozen corpus covers:
- many action IDs aliasing one physical route;
- multiple routes sharing a single power-bus or clock cut set;
- separate mechanical paths with distinct domain manifests;
- distinct route geometry converging to the same recovery region;
- zero-probability but physically possible dead-end branches;
- nonviable intermediate latency states;
- exact minimal cut sets;
- missing/ambiguous domain metadata;
- stale/mismatched upstream evidence;
- input permutation and identifier relabeling;
- new physical branches that must not increase robust route availability;
- routes that exceed the available recovery-energy or horizon budget;
- bounded graph exhaustion and explicit Unknown/Invalid output.

See fixtures/known_answers_v0_1.json. The fixture records are synthetic design tests, not robot measurements.

## 7. Experiment plan: compose with the canonical actuator-fault campaign

Use #4358; do not create a parallel physical fault campaign.

Start with locked-joint and torque-derating cases, then include:
- F22: thermal derating combined with payload and terrain transitions;
- F23: passive overload clutch plus active controller compensation;
- F24: unknown drive fault obscured by encoder bias;
- source-qualified common-cause power, timing and sensing failures.

For each matched run, snapshot the reserve vector:
1. nominal, before injection;
2. immediately after fault onset;
3. after any active diagnostic action and its full latency;
4. after reconfiguration;
5. at the terminal recovery/stop outcome.

Compare nominal, passive-mechanical, controller-only and combined mechanical-plus-controller arms. Keep the fault-unaware negative control. Match morphology, robot/hardware, initial state, task, terrain, payload, fault mechanism/onset/phase/severity, exposure and resource budget. Controller-visible observations must not contain evaluator-only truth.

Evaluate whether the reserve vector predicts measured recovery success, latency, energy and adverse outcomes on held-out fault/terrain combinations. The independent experimental unit is the fault episode/run, not the control tick.

## 8. Research comparators and industry context

- **INSECTRON (npj Robotics, 5 Nov 2026):** centimeter-scale hexapod with magnetic overload couplings, modular replacement and 91–95% recovery across its controlled vertical-perturbation conditions (360 trials). It motivates passive mechanical protection and repairability. Its scale, two-module actuation and perturbation protocol are not directly comparable to a quadruped actuator-fault campaign. DOI: https://doi.org/10.1038/s44182-026-00106-y
- **RAIBO2 (Nature, 2026):** integrated mechanics, motor-driver losses and locomotion-policy design; completed 42.195 km in 4 h 19 min 52 s with reported total CoT 0.25. This is an energy/endurance comparator, not a fault-tolerance result. Reproduction requires compatible mass, speed, energy boundary, terrain and payload. DOI: https://doi.org/10.1038/s41586-026-11102-5
- **LoComposition (arXiv:2606.15896v2, 12 Oct 2026):** separates task reward, hard operational constraints, energy minimization and exteroceptive terrain adaptation; reports 56% lower CoT and 96% fewer operational-limit violations against its own conventional complex-reward baseline, with zero-shot Go2 transfer. These percentages are not transferable leaderboard scores. https://arxiv.org/abs/2606.15896
- **Anticipatory Gait Switching (arXiv:2610.24895, 17 Oct 2026):** look-ahead MDP gait supervision with indoor/outdoor Go2 experiments; compare against reactive and viability-conditioned switching. https://arxiv.org/abs/2610.24895
- **Predictive Safety Curricula (arXiv:2609.37070, 29 Sep 2026):** on ANYmal-D, it reports 63.2% fewer shank-positive crossings than learning-progress sampling across three matched training seeds and 300 crossings per method, but foot scuffs rise from 14.3 ± 1.5 to 29.0 ± 9.6 per 100 crossings because foot contacts are excluded from the targeted safety cost. On a separate production stair-climbing platform, PSC records zero shank-positive trials versus 33/50 for baseline and foot scuffs fall from 14 to 7. Keep these hardware strata distinct and preserve the complete #4326 adverse-event vector. https://arxiv.org/abs/2609.37070
- **Quadruped UED benchmark (arXiv:2610.40364, 29 Oct 2026):** compares 14 curricula on five morphologies and finds no universal winner; in-distribution leaders can differ from held-out-terrain leaders. Use its lesson for independent terrain/fault holdouts, not as a hardware safety baseline. https://arxiv.org/abs/2610.40364
- **Reference-Free Sampling-Based MPC (ICRA 2026):** demonstrates emergent gaits and basic jumping on Go2 hardware, with more complex motions in simulation. Use it as a controller-agnostic candidate where the implementation and resource budget can be reproduced. DOI: https://doi.org/10.1109/ICRA57385.2026.11696779

**Standardized performance:** use ISO 18646-5:2026 for the applicable legged-locomotion performance tests. The standard explicitly does not verify or validate safety requirements. Keep performance testing, fault-recovery experiments and safety/assurance evidence as separate scorecard dimensions. Official standard: https://committee.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/08/68/86850.html

## 9. Proposed targets (not achieved)

### Finite reference qualification
- Exact agreement with independent brute-force enumeration for all bounded known-answer graphs.
- Zero reserve inflation from action aliases, duplicated records, relabeling or equivalent physical paths.
- Zero route acceptance when any action is unadmitted, a possible branch is omitted, an intermediate state is nonviable, evidence is stale, or a hard resource limit is exceeded.
- Exact minimal cut sets for all frozen fixtures.
- Adding physically possible outcomes never increases robust route availability.
- Missing failure-domain identity never produces a positive independence claim.
- Deterministic replay under record permutation and canonical serialization.
- Graph/resource exhaustion returns explicit Unknown/Invalid; never silently truncate.

### Physical experiment objectives
- Before single-fault injection, target a minimum declared cut-set size of two for those recovery goals whose specification requires tolerance of any one listed failure domain.
- After each in-scope single fault, require at least one measured, independently admitted recovery or safe-stop route; otherwise declare the condition outside the qualified capability envelope.
- Show non-inferior task capability and no worsening in any preregistered adverse category against the strongest matched baseline.
- Establish on held-out hardware episodes whether topology-aware reserve improves prediction of recovery outcome, time and energy over raw action counts.
- Publish confidence intervals, exposure, missingness and the full adverse-outcome vector. Zero observed failures are not proof of zero risk.

These are research objectives, not universal safety thresholds. The required cut-set size must be tied to an explicit fault-tolerance requirement and complete failure-domain model.

## 10. Claim ceiling

A successful finite auditor qualifies arithmetic and evidence handling over the supplied finite graph and declared domain manifest. It does not prove that the graph overapproximates the physical robot, that the manifest contains every common cause, that a recovery route is safe in continuous time, that the system tolerates arbitrary faults, or that any robot is certified.

Only matched physical experiments can establish measured recovery behavior for the tested robot, faults, terrain, payload, exposure and operating envelope.