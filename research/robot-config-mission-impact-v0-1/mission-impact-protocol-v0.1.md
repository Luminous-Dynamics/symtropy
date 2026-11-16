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


## 17. Deterministic public-development challenge generator v0.1

The child implementation workstream is [Symtropy #5590](https://github.com/Luminous-Dynamics/symtropy/issues/5590). Its first bounded implementation consists of:
- `challenge-distribution-v0.1.json`: eight change-family strata, weighted allocation, task/site/ODD/severity labels and 23 named coupled factor bundles;
- closed schemas for the distribution profile and generated challenge manifest;
- an isolated Rust generator and `generate` CLI;
- deterministic allocation, domain-separated seed derivation, unbiased bounded integer sampling, atomic bundle selection and canonical SHA-256 digests;
- golden-property tests for repeatability, seed-domain separation, exact stratum allocation, bundle coherence and schema conformance.

### Allocation and replay

For requested count (N), stratum weights (w_i), and total weight (W=\sum_i w_i), v0.1 computes the base allocation \\(\lfloor Nw_i/W\\rfloor\\) and allocates remaining entries by descending fractional remainder with a deterministic seed-derived tie-break. The resulting challenge list is then shuffled with a separate domain-separated PRNG stream. Bundle selection occurs uniformly within the allocated stratum.

The root seed is supplied explicitly. SHA-256 domain separation derives independent deterministic streams for allocation ties, challenge identity, bundle selection and output ordering. SplitMix64 is used as the specified reproducible bounded sampler, with rejection sampling rather than modulo-only reduction. The generator emits exact allocation denominators, seed algorithm, derivation domain, profile SHA-256, per-challenge digest and a manifest digest.

### What v0.1 does not solve

- SplitMix64 is not a cryptographic PRNG; public reproducibility is not evaluator-seed secrecy.
- The 23 factor-profile references are synthetic symbolic labels, not numeric physical parameter ranges.
- Atomic bundle selection prevents independent sampling of coupled axes, but does not prove that the bundle is physically coherent, safe or representative.
- The generator does not synthesize CAD, 3-D environments, robot dynamics, contact physics, sensor observations or calibrated measurement data.
- The current deterministic generator makes no statistical goodness-of-fit, entropy, distribution coverage or physical-scene repeatability claim.

The next generator increment must bind domain-owned, versioned parameter profiles and constraints (payload/CoM/inertia, sensor mount/extrinsics, battery/power/thermal, drive/surface/friction, clock/timebase/latency and safety-chain evidence) and report invalid bundle rejections rather than silently resampling. Only after those profiles have competent domain owners and reference evidence should the phrase “physically coherent distribution” be used without qualification.

### CLI

From the repository root:

```sh
cargo run --manifest-path research/robot-config-mission-impact-v0-1/validator/Cargo.toml --locked -- validate

cargo run --manifest-path research/robot-config-mission-impact-v0-1/validator/Cargo.toml --locked -- generate \
  --seed 0123456789abcdef \
  --count 128 \
  --output /tmp/mip-challenges.json
```

Identical profile bytes, generator version, root seed and count must reproduce the exact canonical manifest digest across supported operating systems. The CI job also exports the lockfile and generated manifest separately for independent comparison.


## 18. Commercial robotics comparison: fair baselines before superiority claims

The companion `commercial-benchmark-crosswalk-v0.1.md` records the current published product and field-trial conditions for Dusty FieldPrinter 2, HP SitePrint and Built Robotics RPD35/RPS25. It separates vendor-declared test conditions from common-referee comparison and from boundary/degradation tests. It also aligns the programme with current NIST mobility/performance measurement work, AMRA-201:2026 and VDA 5050 v3.0.

The key lesson is that a commercial robot's performance is a system: survey/control, CAD/VDC preparation, tracker/total-station setup, software, batteries, consumables, inspection, service and operator workflow all matter. A millimetre tolerance or nominal motor speed does not measure accepted output per workflow crew-hour.

The comparison is not a claim that Luminous outperforms any named product. No Luminous physical robot has been tested against these vendors in this tranche. The first physical campaign must use a task-specific functional unit, independent referee and both:
- reproduction of each product's declared conditions, where feasible; and
- a common-referee matched track under conditions valid for both systems.

Do not force a competitor outside its declared envelope to manufacture an apparent Luminous win. Report operating-envelope applicability and abstention separately from performance.


## 19. Dependency resolution integrity

The isolated validator resolves Cargo dependencies in the focused workflow and exports the resulting Cargo.lock artifact. The lockfile is not committed as a source file in v0.1, so the package pins its expected lockfile digest in a Rust test. The currently verified cross-platform lock artifact is 49,016 bytes with SHA-256:

`874b07517f58aa8453ad012423b8896c0dbe1f180228499cffbbabef7df65490`

If a transitive dependency changes, the golden digest test fails until the dependency change is reviewed and the expected digest intentionally updated. This guards against silent registry drift, but is not a substitute for a committed lockfile in downstream deployment or packaging. CI retains the lock artifact for review.

The corresponding public challenge golden vector fixes:
- profile digest: `03e604e0af5242e9161f2097f687683ed4acd8018202b561e69ba811d5d4b8df`;
- root seed: `0123456789abcdef`;
- count: 128;
- canonical manifest digest: `37059e33d960c69c912ef753e5392cf902bc1a7bb232e11406c597f4eef5b633`.

The three independent Linux/macOS/Windows workflow artifacts have identical manifest bytes and the same Cargo.lock bytes. The generator's golden-vector unit test replays the known-answer vector from the checked-in profile, seed and count.


## 21. Stage-transition development corpus v0.1

The first executable fixture increment is [#5657 — ROBOT-MISSION-STAGE-EVIDENCE-001](https://github.com/Luminous-Dynamics/symtropy/issues/5657). It adds:
- `stage-transition-challenges-v0.1.json`: 17 public synthetic challenges;
- `stage-transition-oracle-v0.1.json`: separate public development dispositions;
- closed Draft 2020-12 schemas for both;
- Rust cross-file checks that bind stage IDs, verifier classes and each case to its expected disposition/reason.

The policy tests only the fixture contract. An advancement requires an applicable observation from an allowed source class, exact configuration identity, current frame/calibration where required, qualified clock mapping where required, a verifier profile and raw artifact. For safety-critical fixtures, the required source-class profile must include an independent referee; quality-critical fixtures require a referee measurement or independent referee. A task/command acknowledgement or robot self-report is not an observation.

The 17 cases include no-evidence command completion, self-report only, a permitted low-risk telemetry postcondition, critical independent-referee requirement, referee rejection, stale configuration, stale frame epoch, expired calibration, unqualified clock mapping, valid independent acceptance without command acknowledgement, authorized optional skip, unauthorized mandatory skip, interrupted resume across configuration revisions, duplicate accepted output replay, contradictory competent observations and an unknown postcondition under safe hold.

This policy is a benchmark comparator, not the only permissible stage design for every robot. The competent stage/claim profile must define which evidence class is appropriate. No physical test, safety authorization or qualification is produced.


## 22. Benchmark validity and long-horizon SOTA comparators

The next benchmark increment should pass a validity audit before its scores are treated as evidence of general capability. Four diagnostics are mandatory:

1. **Shortcut solvability:** a small, weak, non-specialist baseline must not obtain the headline result without the capability being tested.
2. **Statistical significance:** publish denominators, independent units, paired/unpaired structure, uncertainty intervals and preregistered minimum effects. A reported leaderboard difference is not evidence of a real improvement if the design cannot test its significance.
3. **Creeping overfit:** sample from a declared distribution, redraw seeds, include boundary and held-out strata, and detect test-sample and distribution overfitting.
4. **Data-source dependence:** preserve training/source provenance and run source-held-out or contamination-aware tests; a result may measure proximity of training data to test examples rather than generalization.

Jiang et al.'s 2026 benchmark audit identified these four failure modes. In its audit, only 19.8% of LIBERO SOTA claims and 19.7% of SimplerEnv claims were provably statistically significant under the studied evidence. These rates are specific to those benchmark audits, not a universal estimate of robotics research quality. The lesson is to make statistical validity a release gate, not a reporting afterthought. https://arxiv.org/abs/2606.04233

### Long-horizon benchmark comparators

- **LADDER:** 145 tasks across three suites, 1–10 subgoals and 2,024 simulated episodes per system. Its direct low-level policy reports 85.4% on single-subgoal tasks, 26.9% on two-subgoal tasks and 0% on five-subgoal tasks. It finds orchestration helps but physical execution remains a bottleneck; its best listed tool-selection arm reaches 62.0% overall at material API cost. We should adopt horizon scaling and report decision/verification cost, but not borrow household-manipulation scores as construction targets. https://ladderrobotics.ai/research
- **MOST / RoboCasa-Long:** emphasizes memory of task-relevant world-state changes and subtask progress rather than longer raw visual histories. Its reported improvement in maximum task progress on RoboCasa-Long supports explicit progress-state reconstruction and interruption recovery, not a construction success-rate baseline. https://arxiv.org/abs/2611.00734
- **RoboCasa-Shift:** uses matched clean/perturbed rollouts and stage diagnostics. One camera condition changed 8/80 paired outcomes with zero aggregate success-rate change; identical end-to-end degradation from two perturbations had different first-stage failure profiles. Therefore report paired outcome sensitivity and stage progression even where aggregate success does not move. https://arxiv.org/abs/2610.27861
- **DexArena:** 10 long-horizon tasks and 30 stage-level tasks with explicit physically grounded stage success conditions. It reinforces the need for a task-specific stage contract and intermediate progress reporting. https://arxiv.org/abs/2611.02629
- **NIST PAF:** already composes perception, mobility, dexterity and safety component measurements into task-level robotic system performance models. We build on that measurement-science foundation; the MIP contribution is the paired A/B configuration-change, stage-evidence, accepted-output and lifecycle join. https://www.nist.gov/programs-projects/performance-assessment-framework-robotic-systems

### Prospective benchmark targets

These are design targets, not measured results:

- **0** safety- or quality-critical stage advances without required competent, current postcondition evidence.
- **0** duplicate accepted functional units from command/event replay.
- **100%** exact replay of public synthetic fixtures and their declared oracle.
- **≥98%** stage-state reconstruction agreement on a future independently adjudicated, evaluator-held set, with confidence intervals and all ambiguous cases retained.
- **≥95%** correct safe hold/abstention when postcondition evidence, configuration identity, frame, calibration or clock validity is unknown.
- **≥30%** fewer unnecessary re-observations than an all-or-nothing full-reset recovery baseline, provided the critical false-advance rate does not increase.
- **No universal sample count:** determine independent seed/article/lab requirements from a pilot variance model and a preregistered minimum detectable effect. Treat scenes nested within seeds, repeated attempts, episodes on the same article and telemetry frames as clustered observations—not independent robot trials.

A result is only considered better than the matched baseline when it improves the prespecified outcome without violating the evidence, safety, acceptance or statistical-validity gates. Report end-to-end accepted missions, stage-level progress, paired outcome sensitivity, false advances, recovery time, accepted functional units, crew-hours and resource use separately. No compensating weighted score is allowed.


## 23. Reference workcell pilot design

The physical demonstrator design is tracked in [#5662 — ROBOT-REFERENCE-WORKCELL-001](https://github.com/Luminous-Dynamics/symtropy/issues/5662) and documented in `reference-workcell-pilot-v0.1.md`.

The v0.1 scope is one documented wheeled mobile base with two interchangeable modules:
- a survey/inspection sensor assembly (suitable RGB-D, LiDAR and IMU components, individually identified and calibrated);
- a lightweight non-load-bearing layout/measurement tool with an independently measured functional output.

The initial change matrix is: unchanged repeat baseline; camera serial replacement without calibration transfer; sensor remount/extrinsic change; lightweight tool-geometry replacement; compatible battery replacement; and an unknown-dependency negative control that remains software-only. Do not modify brakes, drive, E-stop, safety PLC, protective fields or load-bearing structure in this first campaign. No excavation, piling, lifting, cutting, drilling or occupied-site operation.

The experiment ends at first independently accepted B output, not at first command acknowledgement or machine cycle. It separates module swap, configuration reconciliation, calibration, commissioning, requalification, first-article inspection, elapsed time, total crew-hours, accepted functional units, geometric uncertainty, interventions, energy and recovery.

No hardware has been selected and no physical trial is authorized. Entry gates require the actual base/module bill of materials, site/ODD and risk approvals, verified safe-stop/isolation, independent metrology, #3522 trial receipt, #3531 admissibility, #3505 safety, #5411 transition receipt and #3605 acceptance semantics. Missing owner contracts remain unresolved.
