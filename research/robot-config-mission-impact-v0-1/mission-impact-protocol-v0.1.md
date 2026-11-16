# ROBOT-CONFIG-MISSION-IMPACT-001 — paired stochastic, stage-aware mission protocol v0.1

Status: public synthetic development protocol. No physical robot result or mission qualification is asserted.

Issue: https://github.com/Luminous-Dynamics/symtropy/issues/5547

## 1. What this benchmark measures

This protocol measures the **system-level performance consequences of a robot configuration change**. For the same sampled work package and operating conditions, compare source configuration A with target configuration B:

- Does the complete mission still finish?
- Is the same useful work independently accepted?
- Which stage first diverges, and what failures follow?
- Do intervention, recovery, energy, time, quality or rework change?
- Does the result generalize across challenge seeds, articles, teams and laboratories?

This is deliberately separate from claim applicability:

- #5488 decides which prior claim-specific evidence remains applicable after a configuration delta and proposes requalification obligations.
- #5547 measures what the A-to-B configuration change does to complete-mission performance under matched work.

A preserved claim is not a mission-performance result. A successful mission is not proof that every prior qualification claim remains applicable.

## 2. Current research that changes the protocol

### Stochastic Challenges — CoRL 2026 / arXiv:2611.12804

Gläßer et al. propose standardizing the distribution of reproducible physical scenes instead of one fixed public test set. Their published benchmark reports 568 real-world trials with Stretch 3 and Spot with arm across two laboratories. The authors also disclose that one researcher rebuilt the shared-seed scenes in both labs, so inter-team reconstruction variation is not quantified.

**Adaptation:** publish a seeded challenge distribution, mix paired and independent seeds, estimate seed difficulty separately from repeat noise, and add an independent reconstruction-team factor.

References:
- https://arxiv.org/abs/2611.12804
- https://rpl-bonn.github.io/stochastic-challenges/

### RoboCasa-Shift — arXiv:2610.27861, 20 October 2026

Wang et al. evaluate matched clean/perturbed rollouts where each pair shares the same underlying settings and differs only by a controlled shift. Their report describes 1,040 formal rollouts over four long-horizon tasks, four perturbation factors and three severity levels. It shows that aggregate success can hide paired outcome changes and that equal end-to-end degradation can arise from different stage-failure profiles.

**Adaptation:** pair A/B executions on the same scenario seed; record paired outcome sensitivity even when aggregate success is unchanged; preserve the first observed divergence and downstream failure sequence; do not assume a monotonic performance response to perturbation severity.

Reference: https://arxiv.org/abs/2610.27861

### MOST / RoboCasa-Long — arXiv:2611.00734, 31 October 2026

He et al. argue that long-horizon agents must preserve task-relevant object-state changes and progress, not simply longer visual histories. The paper reports 72.1% versus 65.9% average success on RoboCasa365 for MOST and XR-1, maximum task progress of 42.22% versus 20.83% on RoboCasa-Long, and 43.33% versus 6.66% real-robot success in its reported setup.

These household manipulation results are not construction-robot comparators. The transferable benchmark lesson is to retain consequential state changes and subtask progress over the full mission rather than scoring only the final binary outcome.

Reference: https://arxiv.org/abs/2611.00734

### Existing compositional test science

NIST already composes perception, mobility, dexterity and safety measurements into system-level robotic performance models. We should use that foundation and contribute the paired change-impact, stage-diagnostic and accepted-work layer—not claim to have invented compositional performance assessment.

Reference: https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems

## 3. Benchmark units and pairing

The primary independent unit is a **challenge seed × task/site/ODD instance**. A seed may generate multiple work items, but claims, telemetry frames, controller ticks, retries and stage rows within one mission are not independent physical trials.

For each seed, create a matched A/B pair:
- same task and acceptance profile;
- same site/material/environment stratum;
- same initial state and permitted intervention policy;
- same independent reference method and acceptance boundary;
- same crew/role policy where practical;
- exact source and target configuration/article bindings;
- one declared configuration delta for single-factor cases.

Where hardware permits reversible module changes, counterbalance A/B order and record warm-up, wear, battery state, contamination, calibration and operator learning/carryover. Where the same physical article cannot be used in both conditions, use randomized article/configuration allocation and a hierarchical model. Do not label unpaired article runs as paired.

Use both common seeds, to cancel scene/task difficulty in A/B comparisons, and independent seeds, to estimate generalization across the full distribution. Choose the ratio after a pilot and preregister it; do not select it after seeing which ratio improves a result.

## 4. Mission-stage vocabulary

The stage catalog is profile-bound. This reference decomposition is a starting vocabulary, not a universal process model.

| Stage | Name | Question |
| --- | --- | --- |
| S01 | Task contract | Are work package, preconditions and success criteria understood? |
| S02 | Site observation | Are site, material, obstacles and task-relevant objects observed? |
| S03 | Frame and localization | Are survey control, map/frame epoch and robot pose consistent? |
| S04 | Navigation | Can the robot reach the intended work area? |
| S05 | Base positioning | Can it achieve a useful, accessible and stable work pose? |
| S06 | Tool and calibration | Are module, tool, TCP, calibration and interfaces correct? |
| S07 | Operation | Can the robot perform the declared task? |
| S08 | Independent acceptance | Does output satisfy the functional acceptance profile under independent measurement? |
| S09 | Recovery | Can discrepancies, faults and failed attempts be safely reconciled? |
| S10 | State reconciliation | Are output, residual WIP, configuration and return-to-service states reconciled? |

For each stage transition record:
- stage entry/exit criteria and transition identity;
- event-time and arrival-time ticks, separate clock-domain references, and explicit clock-mapping state (qualified, unqualified or absent);
- attempts, retries, duration and intervention;
- evidence and raw-artifact references;
- status: reached, passed, failed, blocked, not reached, intentionally skipped by policy, aborted, or unknown;
- first observed divergence and later downstream consequences.

Failure classification can be multi-label. FirstDivergenceStage is an observed location in the trace, not causal root-cause proof. Root-cause attribution needs separate evidence and competent adjudication.

## 5. Challenge distribution

Sample at the level of complete configuration/task instances. Stratify across:
- platform family, serial/article and assembly generation;
- tool/end-effector, sensor/mount/calibration, base/drive, payload, battery/power, firmware/controller, safety chain, timebase and frame/site changes;
- task family and functional unit;
- site, surface/material, environment and ODD;
- in-envelope, boundary, out-of-envelope and unknown conditions;
- single-factor changes and preregistered pairwise/three-way interactions;
- task criticality and independent acceptance method.

Couple physically coherent variables. Payload mass, CoM and inertia must be jointly plausible; sensor mount and extrinsics must be represented consistently; battery identity, mass, interface, health and power envelope must agree; drive/tire/track, surface and stopping condition must be sampled as a matched set; site/map/frame epoch changes must carry exact frame context.

Every rejected impossible draw and its reason is retained. Hard cases cannot be silently removed after observing performance.

For scored challenges, publish generator source/version, schema, canonicalization rules, PRNG identity, parameter ranges and sampling procedure. Evaluator-held scored seeds/labels remain private until candidate output is committed; blind custody composes #1300. A hash commitment is not proof of generator correctness or physical-scene authenticity.

## 6. Required comparator arms

| Arm | Description | Why it is needed |
| --- | --- | --- |
| C0 | Atomic subsystem tests only | Shows what isolated tests miss |
| C1 | Aggregate mission success/time only | Measures the cost of lacking stage diagnostics |
| C2 | Fixed public cases | Development baseline; susceptible to memorization |
| C3 | Stochastic independent seeds | Measures distribution-level generalization |
| C4 | Paired stochastic stage-aware evaluation | Proposed benchmark method |
| C5 | Evaluator-only oracle perturbation/stage cause | Diagnostic upper bound; never a candidate |
| C6 | Matched expert/manual workflow | Commercially meaningful total-workflow baseline |

For configuration-change reasoning specifically, preserve #5488's separate preserve-all, invalidate-all, direct-only, generic-adapter, proposed-adapter and finite-oracle comparison arms. Do not conflate the two benchmarks.

## 7. Metrics — no composite score

### Mission and accepted output
- final accepted functional units per robot-hour, direct crew-hour, total workflow crew-hour and elapsed site-hour;
- mission completion, task-progress distribution and first-pass/final acceptance;
- accepted, rejected, reworked, held, aborted, cancelled, superseded and unresolved denominators;
- quality/error distributions, units, uncertainty and acceptance margins.

### Stage diagnostics
- probability of reaching each stage;
- transition success and conditional completion given stage reached;
- first observed divergence, stage-failure progression and multi-label failure counts;
- stage duration, retry and intervention distributions;
- paired outcome sensitivity: fraction of matched seeds whose outcome changes, even if aggregate success is unchanged.

### Resources and human collaboration
- energy and consumables per accepted functional unit;
- setup, survey, calibration, charging, replenishment, idle, waiting and recovery;
- crew/equipment support hours, intervention time and rework;
- damage, material loss and inspection burden.

### Reproducibility and validity
- per-seed and per-stratum results;
- paired effect estimates and intervals;
- between-seed, within-seed, article, team, operator, lab and instrument variance;
- reconstruction fidelity and adjudicator disagreement;
- missing/unknown event intervals and denominator reconciliation;
- shortcut, leakage, overfit, source-dependence and benchmark-gaming probes.

### Separate qualification/lifecycle context
- link #5488 dispositions, but do not treat them as mission success;
- link #5411 reconfiguration chronology, but do not reuse changeover time as task productivity;
- link #3628 maintenance/exposure, but do not equate uptime with accepted output;
- link #11866 terminal reserve, but do not average heterogeneous physical reserves into one score.

Safety, quality, productivity, energy, repairability, reserve and uncertainty stay separate. A hard safety or acceptance violation cannot be compensated by faster cycle time.

## 8. Statistics and reproducibility

A pilot estimates between-seed variance, within-seed attempt variance, article-to-article variation, team reconstruction variation, operator/crew variation, and lab/instrument/calibration variation.

Pre-register per endpoint:
- minimum meaningful difference or equivalence margin;
- type-I error, target power and sample size;
- paired-seed fraction and allocation;
- cluster/hierarchical analysis;
- multiple-endpoint policy;
- missingness, censoring, abort and safety-stop handling.

Report effects and uncertainty. Do not stop early because an interim ranking looks favourable. A result with materially overlapping uncertainty is inconclusive, not a win. More retries on one seed cannot compensate for too few independent seeds.

## 9. Prospective acceptance targets (not achieved results)

- 100% exact source/target configuration, task, seed, ODD, acceptance-profile and evaluator binding—or explicit invalid/unknown.
- 100% scheduled mission time reconciles to a declared stage/state or explicit unknown interval.
- 100% accepted functional units bind independently adjudicated acceptance evidence and raw measurement references.
- 100% failed, aborted, safety-stopped and unknown episodes remain in declared denominators.
- Every primary endpoint reports per-seed outcomes and uncertainty, not pooled success alone.
- Same pinned seed/configuration/task manifest produces deterministic synthetic replay.
- Independent reconstruction teams report fidelity and disagreement; seed determinism is not mistaken for physical reconstruction repeatability.
- Zero synthetic/simulated episodes promoted to physical performance, safety, certification or work-authorization evidence.

Do not set a percentage superiority target until the manual/commercial baseline, instrument capability and pilot variance are measured.

## 10. Physical-test gate

This protocol does not authorize physical trials. A later campaign requires #3505/#3522/#3531/#3591 physical procedure, receipts, admissibility and interlab methodology; task-specific risk assessment, safeguards, qualified operators, site permission and competent independent adjudication; exact configuration and calibration binding; independent calibrated reference measurements suited to the acceptance margin; pre-registered seed allocation and analysis; and reconciliation after interruption, network/power loss or unknown execution outcome.

Start with guarded, low-consequence, reversible work such as stand-off survey/inspection and non-load-bearing layout. No autonomous excavation, lifting, cutting, fastening or public-site operation is authorized by this benchmark. Do not deliberately degrade safety functions or bypass interlocks.

## 11. Composition / ownership

- #5488: claim-specific evidence applicability after a configuration delta.
- #5411: last accepted A to first accepted B reconfiguration chronology.
- #3605: functional unit and accepted-output accounting.
- #3523/#3524: construction-robot task performance profiles.
- #3505/#3522/#3531: physical procedure, receipt and evidence admissibility.
- #3591: cross-lab metrology and reproducibility.
- #3628: accepted production, wear, maintenance, repair and restoration.
- #1834: robot capability/work-package compatibility.
- Symthaea #10543/#4918/#12046: generic change impact, dependency/common-mode lineage and evidence request planning.
- Symthaea #4863/#11866: lifecycle/degradation and productive work versus terminal reserve.

No second capability engine, dependency graph, accepted-output ledger, maintenance ledger, controller, digital-twin truth store or safety authority is introduced.

## 12. Deliverables and claim ceiling

V0.1 artifacts:
1. public stage catalog and mission-pair schema;
2. stochastic matched-pair protocol;
3. synthetic development corpus and separately stored public oracle;
4. cross-file invariants and benchmark-validity checks.

Later:
- evaluator-held challenge and custody protocol;
- Rust-first replay/analysis tool after owner event/receipt contracts are pinned;
- guarded reference workcell campaign;
- second article, independent team/lab replication and public report.

The public artifacts establish methodology and synthetic fixture semantics only. They do not establish robot performance, causal attribution, physical safety, calibration validity, qualification, construction productivity, conformity or vendor superiority.


## 13. Event-time, arrival-time and stage identity

The event trace carries **two different time observations**:
- `event_time_tick` in an explicitly named event clock domain;
- `arrival_time_tick` in an explicitly named receiver/ingestion clock domain.

The trace array is in receiver-arrival order: `arrival_time_tick` must be nondecreasing within its declared receiver clock domain. **Do not require `event_time_tick` to be nondecreasing in that array.** A late or reordered event is precisely the evidence we need to retain. Preserve the original event time, arrival time, both clock domains and array/arrival order; never sort away the discrepancy. Never compare these ticks numerically as if they share an epoch. The trace also carries a clock-mapping reference and an admission state (`qualified`, `unqualified`, or `absent`). A target trace whose event and arrival domains differ and whose mapping is absent/unqualified must force temporal reconciliation and acceptance evidence to remain unknown; it cannot silently be sorted into a supposedly trustworthy timeline.
When the mapping is admitted, translate event time into the receiver/reference timeline using the exact mapping and its uncertainty policy. When mapping is absent, unqualified, stale or contradictory, temporal ordering is unresolved for decisions that depend on it. Do not infer a clock offset from apparent event alignment or from the robot's own summary.

Clock semantics and uncertainty are not owned here. Compose the exact event-time/multirate contract from Symtropy #4947; this protocol only requires both raw clock domains and the mapping/admission reference to survive into the mission receipt.

Every event also binds a nullable `stage_id`. Null means the event is a precondition, cross-cutting record or mission disposition that is not an observation from a reached mission stage. The explicit `event-type-catalog-v0.1.json` binds each event type to its declared stage (or null) and allowed source class. The validator must reject:
- unregistered event types;
- event type/stage mismatches;
- source classes not allowed for the event type;
- stage events emitted from stages labelled `not_reached` or `not_run` in the target oracle;
- missing clock domains or inconsistent clock-mapping reference/admission states.

This avoids inferring stages from event-name prefixes and preserves the distinction between a referee's final mission disposition and an event emitted by an operational stage.

## 14. New long-horizon benchmark comparators checked 16 November 2026

- **RoboCasa-Shift**, arXiv:2610.27861 (20 October 2026), uses strictly matched clean/perturbed rollouts and stage-aware diagnostics. It reports an example where the aggregate success-rate delta is zero although 8 of 80 matched outcomes change, and different perturbations with the same aggregate degradation produce different first-stage failure rates. This is a direct methodological comparator for MIP's paired-outcome-sensitivity control. https://arxiv.org/abs/2610.27861
- **MOST / RoboCasa-Long**, arXiv:2611.00734 (31 October 2026), reports that memory structured around object-state changes and subtask progress improves long-horizon mobile manipulation in its household tasks. MIP should retain task progress and world-state consequences, but it does not benchmark or claim a memory architecture. https://arxiv.org/abs/2611.00734
- **LADDER**, arXiv:2610.18053 (13 October 2026), evaluates 145 long-horizon tasks across three suites with 1–10 subgoals and 2,024 simulated episodes per system; it explicitly measures subgoal progress and compares orchestration approaches. MIP should use task-progress and horizon strata, while adding configuration A/B pairing, physical accepted-output references and lifecycle/qualification joins. The public LADDER leaderboard's model/tool/cost comparisons are specific to its task suites and are not construction-robot performance baselines. https://arxiv.org/abs/2610.18053 and https://ladder.insait.ai
- **Cracking Mobile Manipulation**, arXiv:2610.19600 (14 October 2026), reports an 80% atomic manipulation success rate falling to 19% when composed into mobile manipulation, with failure-guided interventions reaching 49% held-out RoboCasa365 success. MIP should retain stage-specific interventions and the point where failures enter the mission; it should not transplant these household-task percentages into construction claims. https://arxiv.org/abs/2610.19600
- **Robot Manipulation Benchmark Audit**, arXiv:2606.04233 (2 June 2026), identifies shortcut solvability, insufficient statistical significance, creeping overfitting and data-source dependence. Its audit found that only 19.8% and 19.7% of SOTA claims on LIBERO and SimplerEnv, respectively, were provably significant under the authors' analysis. MIP therefore requires low-capacity probes, held-out seeds, per-seed effects and data-source disclosure before any benchmark claim. https://arxiv.org/abs/2606.04233 and https://ripl.github.io/manipulation_benchmark_audit/

The benchmarks differ in robot embodiments, task families, simulator/physical evidence and denominators. Do not compare their success percentages to each other or to construction field productivity. The useful contribution is methodological transfer, not a leaderboard claim.


## 15. Outcome measures and acceptance-margin reporting

The public `mission-outcome-metrics-v0.1.json` file provides one synthetic functional-unit outcome record for each source/target pair. It intentionally records separate quantities:
- attempted, accepted, rejected, held, aborted, unknown and not-started units;
- robot elapsed seconds and total-workflow crew seconds;
- energy in Wh;
- operator interventions and intervention seconds;
- reinspection count;
- signed geometry error, expanded measurement uncertainty and acceptance tolerance, with an independent measurement reference where a result exists.

For the declared synthetic layout tolerance, a result is within the uncertainty-guarded tolerance only when:

`abs(signed_error_mm) + expanded_uncertainty_mm <= acceptance_tolerance_mm`

This is a simplified fixture rule, not a substitute for a task-specific uncertainty budget, decision rule or applicable metrology standard. Real acceptance criteria must be supplied by the relevant owner/customer/referee profile.

The benchmark must retain **distance to the acceptance boundary**, not only pass/fail. MIP-006 intentionally keeps aggregate acceptance unchanged while reducing the target's uncertainty-guarded margin and increasing intervention/reinspection burden. This is the counterexample to a benchmark that equates equal binary success with equivalent operational performance.

No combined score is computed from energy, quality, labour, safety, downtime or requalification effort. Report a vector and its uncertainty. A hard acceptance or safety failure remains non-compensable by throughput.

All numeric values in the current file are fictional fixtures designed to test the contract. They are not measurements, baseline data, or evidence of improved productivity.


## 16. Owner crosswalk and exact interface boundary

`owner-crosswalk-v0.1.json` relates the eight MIP development pairs to the corresponding public RCQ change families:

| MIP pair | RCQ case | Relationship |
| --- | --- | --- |
| MIP-001 camera serial replacement | RCQ-007 | Related change family |
| MIP-002 tool geometry changed | RCQ-004 | Related change family |
| MIP-003 battery health unmeasured | RCQ-009 | Related change family |
| MIP-004 drive module changed | RCQ-010 | Related change family |
| MIP-005 timebase replaced | RCQ-014 | Related change family |
| MIP-006 payload CoM changed | RCQ-005 | Related change family |
| MIP-007 safety chain revision | RCQ-011 | Related change family |
| MIP-008 unknown module | RCQ-016 | Related change family |

Every row is labelled `family_related_not_case_equivalence`. These are not identical conditions, expected dispositions, physical cases or claims. The crosswalk pins the source RCQ corpus by its exact Git blob SHA `9e38da798173c653bd01fc1ceb648fd0ff8a6dff`, not merely by a mutable branch URL.

The other two owner joins are deliberately unresolved:
- #5411's required last-accepted-A to first-accepted-B transition receipt is not yet pinned as a machine-readable artifact in this tranche; `transition_receipt_ref` remains null.
- #3605's accepted functional-unit/inspection contract is not yet pinned as a machine-readable owner artifact here; the current layout acceptance profile is a synthetic fixture only.

The validator enforces these nonclaims. It rejects changing either unresolved owner link into a synthetic "receipt" and rejects mission-to-qualification mappings whose declared mission family no longer matches the MIP corpus. Later owner acceptance should replace these explicit unresolved states with exact, revision/digest-pinned references—not reinterpret the current fixture URIs as real owner contracts.
