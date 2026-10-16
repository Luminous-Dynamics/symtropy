# CIV-RES-IND-001 — Essential-Service Recovery + Small-Industry Restart Benchmark (v0.1)

**Status:** research protocol proposal; synthetic-only; not yet an executable or qualified benchmark  
**Owner:** Symtropy scenario execution and replay  
**Tracking issue:** https://github.com/Luminous-Dynamics/symtropy/issues/1708  
**Date:** 2026-10-16

## 1. Research question

Under the same frozen initial state and declared disruptions, does a bounded recommendation policy help a small community restore essential services and return a workshop to conforming useful output more effectively than transparent non-AI baselines?

The evaluation must allow the candidate to lose. It must distinguish a planning/simulation result from an engineering claim about physical production.

## 2. Compose existing semantics; do not fork them

| Concern | Existing owner / reference |
|---|---|
| Venue-neutral productive-capability closure | Symthaea CIV-BOOT #5774 |
| Shock campaigns and essential-service floors | Symthaea PIE-009E #1788 |
| Future-blind recovery policy under delayed observation | Symthaea PIE-009H #1847 |
| Industrial black-start and restart-criticality/cut sets | Symthaea PIE-009J #1924 and PIE-009K #1932 |
| Production benefit vs conventional baseline | Symthaea MFG-VALUE-001 #7608 |
| Offline logistics and decision records | Mycelix LOG-COMMONS-S0-001 #4901 |
| Spatial alternatives and evidence-bounded intervention analysis | Sol Atlas SOL-ATLAS-004 #21 and PR #116 |

Symtropy executes deterministic scenarios and returns trajectories and receipts. Symthaea may propose and explain plans. Mycelix remains the owner of shared provenance/coordination records and human commitments. Sol Atlas remains a read-only spatial projection. Existing equipment-safety, production-admission and human decision authorities are not replaced.

## 3. Campaign time and conservation rules

A reference implementation must state its temporal convention explicitly. Suggested convention:

1. **Observe:** expose only evidence whose availability step is no later than the current step, including any declared detection delay.
2. **Decide:** a policy receives the observation history through the current step; it does not receive the scenario ID, future shock schedule, or future inventory arrivals.
3. **Validate:** reject plans that violate hard service floors, declared authority, evidence, material/quality, resource, or process constraints.
4. **Commit:** reserve finite resources against one explicit resource ledger; the same lot or energy reserve cannot be spent twice.
5. **Advance:** apply the step's known events and process effects.
6. **Verify:** completed repairs are inspected only after their completion boundary; verified new capacity is usable no earlier than the next step.
7. **Record:** append normalized results and an immutable receipt bound to the input manifest, policy version, model version, and evaluator version.

If an existing authoritative owner defines different semantics, this protocol adapts to that contract rather than silently overriding it. The convention above is a benchmark proposal, not a production scheduler specification.

## 4. Minimal reference fixture

Use a deliberately small synthetic community/workshop, suitable for hand calculation and independent enumeration.

### Essential functions

- potable/process water service;
- sanitation;
- critical electricity;
- food-preservation service;
- basic communications;
- a configurable minimum-care service.

Each function has its own dimensioned demand, lower bound, time interval, evidence quality, and location/population coverage. Values are synthetic fixture constants only. Do not combine unlike units or infer physical quantities from normalized test units.

### Productive capability

The workshop contains explicit capability instances for:
- manual repair and disassembly;
- one bounded subtractive or additive process;
- joining/assembly;
- dimensional inspection;
- a calibration/reference chain;
- one replaceable drive/controller;
- finite repair stock and consumables.

A product route is a dependency graph, not a machine label. A candidate product counts as conforming output only when its material/lot, process revision, required calibration, inspection and authorization evidence all meet the fixture's declared criteria. Geometric production alone is insufficient.

### Required dependency distinctions

> Available machine is not the same as usable capacity.
>
> Usable capacity is not the same as conforming output.
>
> Repairable machine is not the same as completed repair.
>
> Completed repair is not the same as verified capacity.
>
> One replacement part is not productive-equipment closure.
>
> One recovered service does not prove that all groups are served.

## 5. Scenario matrix

Freeze all inputs, event steps, policy settings, expected dispositions and holdout membership before candidate evaluation.

| ID | Injected disruption | Required discriminating behavior |
|---|---|---|
| S00 | Nominal reference operation | Baselines and candidate meet declared service/product criteria or expose the same true infeasibility |
| S01 | Grid outage; finite energy reserve | Preserve hard essential floors before nonessential production; account for reserve depletion |
| S02 | Common-mode fault defeats duplicate units | Redundancy sharing the same fault domain must not count as independent protection |
| S03 | Critical spare import arrives late | Do not consume a future arrival; show blocked capability and feasible local alternatives if any |
| S04 | Calibration/reference unavailable | Reject purportedly conforming output where calibration is a prerequisite, even if the machine runs |
| S05 | Fabricated part has wrong material grade | Product geometry must not bypass material/quality evidence |
| S06 | Delayed and contradictory observations | Policy actions depend only on currently available observations; contradiction/unknown stays visible |
| S07 | Skill and maintenance time are bottlenecks | Count scarce technician time once; report incomplete repairs and residual service impacts |
| S08 | Unequal demand across locations | Report service shortfall by location/group, not only total delivered volume |
| S09 | Cold restart after shutdown | Include black-start prerequisites, commissioning time, finite spares and post-repair verification |
| S10 | Benign workload held out from policy tuning | Evaluate generalization without changing constraints or selecting the holdout after results are known |

Add pairwise and selected compound combinations only after single-shock semantics are independently checked. Do not infer a disruption probability from scenario counts.

## 6. Policy comparison

- **P0 — documented conventional procedure:** fixed priorities and explicit decision rules.
- **P1 — transparent criticality heuristic:** rank actions by declared essential-floor protection and known blocker removal; all tie-breaking is explicit.
- **P2 — Symthaea recommendation:** generate candidate actions/plans with assumptions and expected resource/capability deltas; subject proposals to the same feasibility, evidence and authority validator as P0/P1.
- **ORACLE — diagnostic only:** exhaustive or future-informed upper bound for tiny fixtures, clearly marked non-deployable. It is not a fair real-time competitor.

Freeze policy versions and parameter choices before holdout scoring. Keep P0/P1 as serious baselines rather than straw men. Component ablations may be added, but a combined system is not assumed to improve outcomes.

## 7. Metrics and hard guardrails

Publish every measure separately. At minimum:
- step of first essential-floor breach; time to restore each essential function; per-step service deficits;
- coverage and unmet demand by location/population group;
- step of first conforming product after restart; conforming output; rejected output; scrap/rework;
- remaining required-capability closure and critical dependency cut sets;
- resource ledger: energy, repair stock, consumables, imports and labor/skill time used;
- invalid or blocked plans, wrong recovery claims, false-positive repairs and human-review requests;
- time below essential floors, time to verified productive capacity, and policy regret against the labeled small-fixture oracle;
- exact replay agreement, input-order invariance and independent-evaluator agreement.

**Hard gates:** no unsafe/unqualified product counts as useful output; essential service and quality failures cannot be offset by faster throughput; a missing fact cannot become a favorable default; policy failure is a valid benchmark result.

Do not create a composite civilization score, resilience score, or hidden weighted total. If a user-supplied weighting is later used, publish it as a separate view while retaining the full outcome vector and all hard-gate violations.

## 8. Independent-oracle and negative-control design

The reference evaluator must be structurally independent of production aggregation code where practical. For tiny scenarios, enumerate feasible plan choices and recompute resource consumption, service delivery, restart step and reported metric values directly from canonical fixture inputs.

Include mutation controls for:
- missing or fabricated receipt;
- changed input manifest with unchanged run ID;
- changed policy/evaluator version;
- duplicate-spent inventory or energy;
- repair applied retroactively in the same step;
- future import consumed early;
- stale calibration treated as current;
- unknown capacity interpreted as available;
- changed scenario ID that alters earlier actions despite identical observable history;
- reordered inputs changing normalized results;
- failed scenarios omitted from aggregate reports;
- more severe constraints producing a better declared conservative result without an explicit changed assumption.

Any evaluator that trusts the candidate's summary instead of independently recomputing the result is not an independent oracle.

## 9. Acceptance ladder

- **H0 — frozen protocol:** scenario schema, unit contract, temporal semantics, policy contracts, exact corpus identity, expected outcomes and negative controls are reviewed.
- **H1 — deterministic reference execution:** same manifest/policy/model yields stable normalized trajectories and receipts.
- **H2 — independent verification:** reference evaluator recomputes the core ledger and rejects mutation controls.
- **H3 — fair policy comparison:** report every policy against identical cases; show wins, losses, ties, failures, hard-gate violations and inconclusive results.
- **H4 — source-qualified replay:** only when lawful and quality-controlled operating data exist; synthetic/simulated/replay/physical evidence remain separate.
- **H5 — bounded shadow pilot:** requires domain-expert review, a preregistered primary outcome, existing operator authority and machine-safety systems. No automated actuation or dispatch is introduced by this benchmark.

Each run records exact code SHA, input/corpus digest, policy and evaluator revision, scenario IDs, assumptions, exclusions, all failures and claim ceiling. A queued CI job is not a pass. A local or Nix-hermetic run can provide additional evidence only when its exact subject and outputs are preserved.

## 10. Research grounding

- NIST, [Community Resilience Planning Guides](https://www.nist.gov/programs-projects/development-and-implementation-community-resilience-planning-guides): infrastructure interdependencies, cascading consequences and timely recovery of social/economic function.
- NIST, [Digital Twins for Advanced Manufacturing](https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing): interoperability and verification, validation and uncertainty quantification (VVUQ).
- NIST, [Digital Twins Workshops Summary Report (published 2026-07-21)](https://www.nist.gov/publications/digital-twins-workshops-summary-report): outstanding industry-wide challenges include interoperability, VVUQ, cybersecurity and workforce readiness.
- [Decentralized Spare Parts Production for the Aftermarket using Additive Manufacturing: A Literature Review](https://doi.org/10.1016/j.procir.2022.05.081): local small-batch spare production is a supply-network design option, not proof that a particular part or process is technically qualified.

Research is motivation for the test design, not validation of our simulator or evidence of measured gains.

## 11. Claim ceiling

H0–H3 establish only the declared synthetic semantics and the comparative behavior of the tested policies in this bounded corpus. They do not establish physical manufacturing performance, site self-sufficiency, economic savings, safe plant restart, or civilization-scale resilience. Any wider claim requires a separately admitted evidence tier and domain-expert review.
