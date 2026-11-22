# Mobility assurance benchmark and SOTA position v0.1

Date: 22 November 2026  
Tracks: #6669 authority-linearized mutation; #6698 verified action composition; #6743 dependency-component quarantine.  
Status: benchmark design and target proposal. No SOTA result, robot run, simulation result, HIL result or physical qualification is claimed.

## 1. What field should this compete in?

Do not compare the dependency-quarantine reference directly with VLA manipulation success rates. They are different layers:

- Generalist policy benchmarks measure whether a robot policy completes tasks and generalizes across scenes.
- Task-aware safety benchmarks measure whether execution violates physical/task-specific safety specifications.
- Task-and-motion planning benchmarks measure plan feasibility, resource/precedence constraints and the productivity benefits of parallel execution.
- Symtropy's target layer measures whether an exact action has current authority at mutation, whether simultaneous actions are provably non-interfering, whether outcome uncertainty is contained to a complete dependency closure, and whether the result is independently reconstructible.

Our work should be a portable assurance/evidence layer evaluated on traces from those benchmarks, not a replacement for their policy metrics.

## 2. Current external comparators

### Generalist manipulation: RoboCasa365

RoboCasa365 covers 365 everyday tasks over 2,500 diverse kitchen environments, with 600+ hours of human demonstrations and 1,600+ hours of synthetic demonstrations. Its public multi-task leaderboard reports a 50-task evaluation split with Atomic-Seen, Composite-Seen and Composite-Unseen task groups.

Use it for household action diversity, scene variation, long-horizon composite tasks and policy baselines. It does not by itself measure authority freshness, dependency completeness, native mutation fencing or effect reconciliation.

References:
- ICLR 2026 conference paper: https://proceedings.iclr.cc/paper_files/paper/2026/hash/a05003fdb1e9562ab0c0a9719ea4de10-Abstract-Conference.html
- Official benchmark overview: https://robocasa.ai/docs/introduction/overview.html
- Public leaderboard methodology: https://robocasa.ai/leaderboard

### Task-aware safety: SafeVLA-Bench

The official leaderboard snapshot states 24 policies, five evaluation suites, 22,500 episodes and eight safety specifications, last updated 26 September 2026. It separates:
- SR: native task success;
- Safety: episodes satisfying every applicable task-aware specification;
- SBU: successful episodes that still violate a specification;
- VSI: worst-violation severity.

On RoboCasa-365 Atomic-Seen in that snapshot, Xiaomi-Robotics-1 has 81.9% native success but 50.1% safety and 37.7% successful-but-unsafe episodes. The highest-safety listed policy in that table has 53.1% safety, 58.4% success and 24.0% SBU. This makes the central point: binary task success is not a safety argument.

Use SafeVLA's exact rollout logs and task-aware STL outputs as a downstream challenge set. Add our assurance trace fields without changing the host policy, task, observation, action, seed or evaluator. Do not conflate a software quarantine pass with a policy's physical safety result.

References:
- Official leaderboard and current table: https://safevla.org/leaderboard.html
- Benchmark method and metric definitions: https://safevla.org/
- Paper: https://arxiv.org/abs/2606.00773

### Multi-robot parallel task and motion planning

A 2026 heterogeneous dual-robot TAMP study applies SAT-based post-planning interleaving with geometric validation. It reports up to 19.8% simulation execution-time reduction and 20–25% on its dual-Franka hardware tasks. This is the relevant productivity comparator for concurrency, but its platform/tasks/evaluator are not a matched Symtropy baseline and it does not establish this work's authority/effect guarantees.

Our target should be to reproduce that class of task and protocol, then exceed it on verified makespan while holding the safety/evidence gates fixed—not by running a different task or weakening checks.

Reference: https://www.sciencedirect.com/science/article/pii/S0921889026004446

### Measurement science and safety frameworks

- NIST's Mobility Performance of Robotic Systems programme explicitly develops metrics, test methods, artifacts, datasets, testbeds and measurement systems for stability, adaptability, task efficiency, completion quality and assurance. This is a strong fit for our metrology/benchmark design: https://www.nist.gov/programs-projects/mobility-performance-robotic-systems
- ISO 21448:2022 is a SOTIF framework for road-vehicle functions relying on situational awareness. ISO currently lists a second-edition working draft under development. Use its functional-insufficiency reasoning as a reference where applicable, not a blanket mobile-robot compliance claim: https://www.iso.org/standard/77490.html and https://www.iso.org/standard/93071.html
- ISO 34502:2022 is a scenario-based ADS safety evaluation framework scoped to specified road-vehicle/limited-access-highway use; its scope excludes misuse, HMI and cybersecurity. It is an adjacent scenario methodology, not an automatic standard for construction or household robots: https://www.iso.org/standard/78951.html
- UL 4600 Edition 3 is active and concerns safety principles, risk mitigation and lifecycle safety arguments for autonomous products. It is a safety-case reference, not evidence that this project is certified: https://www.shopulstandards.com/ProductDetail.aspx?productId=UL4600

## 3. Benchmark arms

Keep task, robots, controller, policy, sensor privilege, task seed, resource budget and evaluator fixed.

A. **Mission-wide hold:** current conservative behavior after any unresolved effect.  
B. **Pairwise-only dependency graph:** diagnostic comparator showing what is lost when hyperedges/common causes are omitted; never authorize operation.  
C. **Owner-qualified batch composition:** #6699 exact footprints, occupancy/resource owner checks, authority context and aggregate capacity.  
D. **Composition plus dependency quarantine:** #6743 exact-scope owner inventories and transitive hypergraph closure; global hold on any incomplete or unknown dependency.  
E. **Naive parallel:** isolated simulation diagnostic only, never a product candidate or real actuator permission.

This sequence isolates value from planning, geometry, aggregate constraints, authority fencing and uncertainty containment instead of attributing all improvement to one opaque score.

## 4. Non-compensatory benchmark targets

### Gate 0 — deterministic software oracle

Hard acceptance:
- zero under-quarantine in the frozen public corpus and evaluator-held adversarial set;
- 100% agreement with an independently implemented exact graph/hypergraph closure oracle;
- 100% whole-scope hold for missing, unknown, incomplete, malformed, stale-scope, stale-attempt or unknown-impact owner evidence;
- every action attempt has exactly one outcome classification; missing, duplicate or foreign outcomes block;
- no action outside the computed quarantine is described as released, authorized or safe to execute.

### Gate 1 — inventory/coverage integrity

Measure per owner and per dependency kind:
- exact action-scope coverage;
- inventory freshness and commitment lineage;
- declared versus observed domain counts;
- number of actions and total hyperedge incidences;
- unknown/incomplete/ambiguous rates;
- evaluator-discovered omitted dependencies;
- common-cause and cross-domain overlap.

A "complete" flag is an owner assertion, not self-authenticating truth. Qualification requires owner-specific completeness arguments, source-coverage tests and evaluator-held mutation cases.

### Gate 2 — held-out isolation quality

Research hypotheses, only on a preregistered supported profile:
- containment precision ≥95%;
- isolation coverage ≥95%;
- zero under-quarantine;
- 100% global fallback whenever closure cannot be reconstructed.

Precision and coverage are secondary objectives; neither can compensate for under-quarantine. Publish per-domain and per-stratum numerators/denominators, not only averages.

### Gate 3 — complexity and resource cost

Measure deterministic operation counts, memory and latency at 10, 100, 1,000 and 10,000 actions. Include:
- long chains and branching dependency trees;
- one giant capacity/observer hyperedge;
- many overlapping hyperedges;
- disjoint components;
- many unresolved seeds;
- worst-case all-scope invalidation;
- large numbers of domain memberships.

The reference implementation uses incidence traversal and visits each domain at most once. The expected bound is approximately O((A + I) log A), where A is actions and I is total action/domain incidence because of ordered indexing; benchmark actual operations, allocations and memory rather than advertising an unmeasured latency result.

### Gate 4 — matched operational productivity

After authentic owner integration and exact native fencing, target:
- ≥25% median verified-makespan reduction versus the mission-wide serial baseline on a preregistered independent-action subset;
- ≥30% stretch target on a matched heterogeneous dual-robot subset;
- no worsening of any hard safety, authority, false-completion, effect-attribution or reconciliation gate;
- publish throughput per robot-hour, verified work per energy unit, idle time, operator interventions, rework and quarantine duration separately.

The 25% target is designed to meet or exceed the 20–25% hardware reduction reported by the 2026 dual-robot TAMP comparator. A superiority claim requires reproducing its task distribution or another fully matched baseline, hardware, controller, policy, reservation assumptions and evaluator. Until then, it is a target only.

## 5. Statistical design

The independent unit must be a staged dependency/outcome event or a complete task episode—not individual frames, messages or repeated controller ticks from one event.

Freeze exact source commits, owner schemas, task/seed schedule, scenario manifest, evaluator version and exclusion rules before confirmatory runs. Keep evaluator-held truth separate from candidate-visible owner inventories. Use paired task seeds and cluster-aware confidence intervals for repeated tasks/robots/sites.

For zero observed failures in n independent Bernoulli trials, a one-sided 95% exact upper bound is approximately 3/n for small probabilities. In particular, about 2,995 independent zero-failure trials are needed for an upper bound below 0.1%. This is not a universal certification threshold; correlation or clustered trials reduce effective sample size and require an appropriate model.

A cancelled, skipped, queued, stale-head or incomplete run is not a pass. Public fixtures are conformance examples, not a substitute for hidden evaluator-held cases.

## 6. Failure mutations the benchmark must include

- Remove one state read/write dependency.
- Omit a shared power, thermal, battery, bandwidth or lifting-capacity hyperedge.
- Omit a shared observer, calibration, localization, clock or boot failure domain.
- Replace one action attempt/ticket digest with a stale or concurrent replay.
- Change object generation, platform configuration, ODD, mode or authority epoch.
- Drop, duplicate, reorder or delay a physical-effect outcome.
- Mark an inventory complete while omitting a staged hidden dependency.
- Supply a dependency domain with unknown impact or a member outside the exact scope.
- Cause the native application to occur and then lose the acknowledgement.
- Reconcile one action but leave a dependent precondition stale.
- Change shared authority/clock state while apparently disjoint components remain.

Mutation score, detected omissions, global-fallback correctness, false holds and closure accuracy are reported separately. A passing validator on supplied inputs cannot prove that the supplied dependency inventory is physically complete.

## 7. The route to outperforming SOTA

The differentiator is not a faster trajectory planner by itself. It is **assurance-preserving work conservation**:
1. Prove which exact actions are dependent, including hyperedges and common causes.
2. Linearize authority at the native mutation boundary (#6669).
3. Compose only fully qualified independent batches (#6699).
4. When outcomes are unresolved, quarantine only a complete dependency closure (#6743); otherwise retain global hold.
5. Reconcile each physical effect independently and rebuild world-state preconditions before new work.
6. Evaluate productivity only over independently verified completed work.

This is how we avoid optimizing speed at the expense of correctness. The #6713 common-cause sensing work is a candidate upstream research input, but its synthetic probabilities are not evidence of actual sensor independence or dependency completeness.

No policy-leaderboard, physical safety, regulatory or SOTA superiority claim is made by this benchmark design.
