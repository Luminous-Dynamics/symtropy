# Cross-Domain Autonomy Assurance Benchmark (CDAAB) — Research and targets v0.2

**Snapshot:** 2026-11-21  
**Status:** research protocol and target values, not measured results  
**Owner boundary:** Symtropy supplies the action/effect and mission-assurance seams; Mycelix owns identity, evidence lineage, time correlation and trust relationships; the domain owner supplies controller, ODD, navigation integrity, runtime assurance, physics and independent physical observation.

## 1. The paradigm shift: benchmark the assurance of the action, not the fluency of the agent

A conventional task score answers: *did the system finish this task in this benchmark run?* That is necessary, but not enough for an autonomous system that changes the physical world.

CDAAB asks five separate questions:

1. **Can it act?** Does it accomplish the task under matched environment, embodiment and budget constraints?
2. **Does it know what happened?** Does independent, fresh, configuration-bound evidence establish the actual effect?
3. **Does it know when it does not know?** Does it hold, observe or reconcile rather than fabricate certainty?
4. **Can it recover safely?** Does it handle no-effect, partial effect, delayed evidence, resets, partitions, clock changes and common-cause failures without blind retries?
5. **Can an independent party reproduce and audit the claim?** Are the plan, scenario set, source, build, runtime, evaluator and episode identities bound before execution?

The benchmark is an assurance overlay, not a replacement for LADDER, Bench2Drive, RLBench, BEHAVIOR-1K, or domain-specific flight and maritime evaluations. Those suites remain the domain performance baselines; CDAAB tests whether the system's action, observation, recovery and result claims are trustworthy.

## 2. Comparison to current public baselines

| Baseline | Published scope and reported result | What CDAAB adds; comparability warning |
|---|---|---|
| [LADDER](https://ladderrobotics.ai/research) | 145 long-horizon tasks, 1–10 subgoals, three suites, two embodiments and 2,024 simulated episodes per system. GPT-Policy with GPT-6 Astra reports 62.0% overall task success and $6.49 API cost per successful episode. | Match full-suite task set, embodiment, simulator, policy budget and evaluator version. Report 8–10 subgoal performance, verified effect, recovery, cost and energy. The public 71.5% Gemini 4 Carbon figure is LADDER-Lite's 10-task subset and is not comparable to full-suite Overall. |
| [Bench2Drive v0.0.4](https://github.com/Thinklab-SJTU/Bench2Drive/blob/0.0.4/docs/v004_update.md) | 220 validation routes (44 scenarios × 5). Published LEAD result: 95.59 Driving Score and 87.28% Success Rate. | Compare only on the same 0.0.4 route/evaluator set. Recompute underlying event metrics independently; audit stationary time, controller overrides, route leakage and score aggregation. Driving Score is not interchangeable with task success or physical safety. |
| [RLBench / BEHAVIOR-1K](https://hai.stanford.edu/assets/files/ai_index_report_2026.pdf) | Stanford AI Index 2026 reports 89.4% for EquAct on an 18-task RLBench subset; BEHAVIOR-1K contains 1,000 household activities in simulation. | Use as manipulation and household generalization comparators, not as safety qualification. Include task horizon, embodiment, scene variation, demonstrations and success semantics in every comparison. |
| [IEEE 1588-2019 PTP](https://standards.ieee.org/ieee/1547/6825/) | Active precision-clock synchronization standard; its profiles support heterogeneous networked clocks, with sub-microsecond synchronization and sub-nanosecond time transfer possible in properly engineered networks. | Clock synchronization is not source truth, clock identity, correlation provenance, event causality or proof that a timestamp is admissible. |
| [NASA SPICE SCLK](https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/sclk.html) | Spacecraft-clock conversions depend on spacecraft-specific SCLK kernel parameters, partitions and mapping coefficients. | Preserve the source clock/partition, destination scale/epoch, kernel/profile commitment, validity and uncertainty. Do not treat raw spacecraft or monotonic ticks as UTC or milliseconds. |
| [ISO 34502:2022](https://www.iso.org/standard/78951.html) | Scenario-based ADS safety evaluation guidance for limited-access highways. Its published scope excludes misuse, HMI and cybersecurity, and non-safety comfort, energy and traffic-flow efficiency. | Add adversarial evidence, cyber/common-cause, recovery, HMI/override and cost/energy dimensions rather than treating one scenario score as a complete system safety argument. |

The LADDER leaderboard distinguishes the full three-suite result from LADDER-Lite and reports 95% Wilson intervals. It also shows that long-horizon physical execution remains difficult and expensive. This is a reason to report matched task success and cost frontiers, not to collapse all systems into one scalar score.

## 3. Benchmark validity is itself a qualification gate

A 2026 audit of manipulation benchmarks identifies shortcut solvability, insufficient statistical significance, creeping overfit and data-source dependence as ways a leaderboard can misrepresent general capability. It reports that only 19.8% of the audited LIBERO SOTA claims were provably statistically significant. CDAAB therefore requires an evaluator-integrity report before performance claims are eligible.

Required checks:

- Hidden evaluator truth is inaccessible to the candidate/controller. The public controller-visible manifest does not include hidden fault identities, expected outcomes or private test labels.
- Plan digest, scenario manifest, evaluator version, source commit, artifact, runtime, hardware, randomization commitment and exact episode IDs are frozen before candidate execution.
- A separate evaluator recomputes outcomes from event logs and independent observer evidence. The controller cannot author its own success label.
- Negative controls include stale/replayed receipts, event-ID collisions, source substitution, hidden controller override, stationary/no-progress exploitation, duplicated seeds, out-of-manifest episodes, post-hoc exclusion reason codes, common-cause observer/executor dependencies and metric-denominator manipulation.
- Positive controls establish that valid, well-formed outcomes are accepted; negative-only test sets are not sufficient.
- Results report per-stratum denominators, exclusions, confidence intervals and failed/incomplete/aborted runs. A preflight abort is a recorded non-complete run, never a successful episode and never silently discarded.

Reference reading: Jiang et al., [What Are We Actually Benchmarking in Robot Manipulation?](https://arxiv.org/abs/2606.04233) (2026).

## 4. Non-compensatory metric families

Do not publish a weighted aggregate that allows higher task success to compensate for false completion or an unsafe retry. Publish a dashboard and a gate state.

### A. Contract and admission integrity

- Invalid ticket admission rate.
- Stale mission/action/attempt acceptance rate.
- Configuration, ODD, mode and authority mismatch acceptance rate.
- Clock-basis, epoch, validity-window and correlation-commitment mismatch acceptance rate.
- Replay, duplicate-ID/different-digest and sequence-regression acceptance rate.
- Common-cause observer/executor acceptance rate.
- Unknown or unqualified time projected into an actionable receipt.

**Hard target:** 0 accepted invalid mutations in the deterministic negative-control corpus; 100% rejection of each enumerated negative control; 100% acceptance of valid positive controls. Report each mutation family separately.

### B. Physical outcome correctness

- False completion rate: M1 says satisfied when independent physical ground truth says not satisfied.
- Missed completion rate: valid completed effect incorrectly held or rejected.
- Partial-effect detection and classification accuracy.
- Unknown-outcome detection and time-to-reconciliation.
- Observer source independence and calibration qualification.
- Event-to-receipt latency and evidence freshness at the mutation boundary.

**Hard target:** 0 false completion in the pre-registered safety-critical negative-control corpus. Report confidence bounds and exposure; do not infer a zero operational hazard rate from a finite test set.

### C. Task performance and long-horizon reliability

- Full-task success and subgoal progress.
- Success by horizon (1–2, 3–5, 6–7, 8–10 subgoals).
- Success by task category, embodiment, environment and ODD stratum.
- Recovery-adjusted completion, partial-effect rate and unknown-outcome rate.
- Collision, damage, constraint violation and safety-stop rates.

**Research target:** exceed the current full-suite LADDER comparator by at least 8 percentage points while retaining independent-effect evidence and matched hardware/policy budgets; exceed the current 8–10 subgoal stratum baseline by at least 15 points. These are targets, not achieved results.

### D. Recovery and resilience

- Unsafe automatic retry count.
- Correct hold/reconciliation/recovery classification.
- Recovery success and time-to-safe-state after injected faults.
- Partition behaviour: local safety maintained without federation connectivity.
- Recovery-plan and authority-epoch correctness.
- Mission progress retained without falsely completing unverified subgoals.

**Hard target:** zero blind retries following unknown or partial physical outcomes; 100% correct non-success disposition in the deterministic injected-fault corpus.

### E. Efficiency and economics

- Dollar cost per independently verified completion.
- Energy (Wh) per verified completion.
- CPU/GPU seconds, VLM calls, sensor queries, network bytes and data volume.
- Median and p95 action-to-observation, reconciliation and recovery latency.
- Operator interventions per verified completion.
- Pareto frontier of verified task success vs cost, energy and latency.

**Research target:** at least 2× verified completions per dollar and 20% lower sensing/compute energy per verified completion than the matched baseline, without degrading safety/evidence gates. Publish raw values; do not hide cost behind model-only pricing.

### F. Generalization and statistical validity

- Held-out scene/object/environment performance.
- Cross-embodiment and cross-domain transfer.
- Fault family generalization; unseen combinations of known faults.
- Confidence calibration, selective risk and abstention quality.
- Effect size, cluster-aware confidence interval and power to detect the pre-registered minimum meaningful difference.

Use paired comparisons where the same task/seed can be run across systems. Cluster uncertainty by task/scenario, environment, robot/embodiment and seed as appropriate. Preregister the minimum detectable effect and sampling plan; report interval estimates and the number of independent clusters. Do not choose sample counts after looking at outcomes.

## 5. Matched baseline ladder

Evaluate all candidates against the same policy, platform and budget where possible:

- **A — Direct policy:** the underlying policy/controller without additional decomposition or evidence orchestration.
- **B — Planner + conventional retries:** subgoal planner with the baseline task-success evaluator.
- **C — Evidence-gated actions:** action ticket, configuration/authority admission, independent physical-effect evidence, no-success-on-acknowledgement and fail-closed unknown outcomes.
- **D — Causal recovery:** C plus exact action-attempt binding, event identity, reconciliation, explicit safe recovery permits and deterministic replay.
- **E — Active assurance:** D plus value-of-information sensing, risk-budgeted observation scheduling and causal diagnostics that choose the next observation or recovery action.

Keep the controller constant when comparing assurance layers. Separately report model/controller changes. The difference between C/D/E and A/B estimates the value of the assurance architecture, rather than mixing that value with a stronger model.

## 6. Paradigm-shift research hypotheses

These are research directions, not claims of implementation or SOTA performance.

1. **Proof-carrying actions:** a physical mutation is represented by an immutable action ticket binding platform/configuration, ODD/mode, authority, navigation evidence, clock basis, expected effect and expected observer failure domains.
2. **Observation is a first-class control action:** after a mutation, the system chooses the next sensor/observer query by expected value of information, risk reduction, latency and energy—not by a fixed periodic schedule.
3. **Causal recovery instead of retries:** unknown outcome triggers reconciliation; a confirmed no-effect outcome triggers a separately authorized recovery plan; partial effect triggers re-planning from observed state.
4. **Common-cause-aware sensing:** reliability comes from dependency diversity and independently qualified observer paths, not simply adding more sensors or multiplying confidence scores.
5. **Cross-domain assurance substrate:** the same mission/evidence semantics can be specialized by road, mobile manipulation, marine and aerial ODD owners without inventing a universal controller or diluting domain safety rules.
6. **Benchmark the benchmark:** evaluator provenance, hidden-test isolation, score recomputation, shortcut attacks and statistical validity are independently tested before a system is allowed onto the performance leaderboard.

## 7. Assurance ladder and evidence labels

| Level | Evidence | What it supports | What it does not support |
|---|---|---|---|
| L0 | Schema/type/contract tests | Input and transition invariants under the declared model | Simulator validity or physical correctness |
| L1 | Mutation, differential-oracle, fuzz and deterministic replay tests | Robustness of software contracts and evaluator rules | Real device/observer behaviour |
| L2 | Fault-injected simulation with independent ground truth | Performance in the named simulator and scenario corpus | HIL or physical safety |
| L3 | HIL with hardware clocks, sensors and actuators | Hardware/software interface and timing evidence in the tested configuration | Open-world field performance |
| L4 | Controlled physical trials under approved ODD and oversight | Measured performance in the tested physical conditions | General certification or unrestricted autonomy |
| L5 | Independent safety case, regulatory approvals and operational monitoring | Only the explicitly scoped deployment claim | Safety outside the approved scope |

Every report must label evidence as **implemented**, **structurally checked**, **targeted CI passed**, **broad CI passed**, **simulated**, **HIL**, **physical**, or **independently/regulatorily qualified**. No level is implied by a green software workflow.

## 8. Initial target ledger

| Measure | Target | Evidence required before status can change |
|---|---:|---|
| Invalid action-ticket acceptance | 0 | Independent negative-control oracle |
| False completion in safety-critical negative controls | 0 | Independent physical/evaluator truth set |
| Unknown/partial effect blind retries | 0 | Fault-injected run receipts and replay |
| Stale attempt acceptance | 0 | M1/M2 cross-stack mutation corpus |
| Observer/executor common-cause acceptance | 0 | Authenticated dependency inventory and adversarial mutations |
| Positive-control acceptance | ≥99% | Preregistered valid control corpus and confidence interval |
| Full LADDER success | ≥70% research target | Full 145-task suite, matched embodiment and cost |
| LADDER 8–10 subgoal success | ≥50% research target | Full suite and per-horizon reporting |
| Bench2Drive v0.0.4 | Beat LEAD's 95.59 DS / 87.28% SR | Same 220 routes, versioned evaluator and independent metric audit |
| Verified completions per dollar | ≥2× matched baseline | Full cost accounting, not API token cost alone |
| Sensing/compute energy per verified completion | ≥20% below matched baseline | Measured hardware power, same tasks and assurance gates |
| Safe recovery after injected faults | ≥90% research target; 0 unsafe recovery | Fault family coverage and independent ground truth |

Targets may be revised only before a confirmatory evaluation plan is frozen, with the reason and new target version recorded.

## 9. References

- LADDER benchmark, full research and leaderboard: https://ladderrobotics.ai/research
- Bench2Drive v0.0.4 evaluation release: https://github.com/Thinklab-SJTU/Bench2Drive/blob/0.0.4/docs/v004_update.md
- Stanford AI Index Report 2026: https://hai.stanford.edu/assets/files/ai_index_report_2026.pdf
- Jiang et al., benchmark validity diagnostics (2026): https://arxiv.org/abs/2606.04233
- IEEE 1588-2019: https://standards.ieee.org/ieee/1547/6825/
- NASA NAIF SPICE SCLK: https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/sclk.html
- ISO 34502:2022: https://www.iso.org/standard/78951.html
