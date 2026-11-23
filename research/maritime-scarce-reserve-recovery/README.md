# Maritime scarce-reserve multi-vessel recovery benchmark v0.1

A standalone, synthetic-first Rust benchmark for **two simultaneous vessel/berth disruptions competing for one reserve berth**, with participant-specific authorization, critical onward connections, costly observation, common-cause evidence controls and an independent finite-policy oracle.

This is a new P0 reference benchmark. It does not replace the port-call planner, TOS, berth optimizer, Mycelix authorization owner, event-association owner or generic VOI engine.

## Why this benchmark exists

Terminal optimization research has substantially advanced beyond single-vessel equipment scheduling. Recent work addresses robust integrated berth/quay-crane/yard planning, OPS/energy dispatch, and multi-ship mooring. The remaining research opportunity is to test how a system chooses and justifies scarce recovery capacity when:
- multiple vessels are disrupted simultaneously;
- the reserve asset is itself unavailable in some possible worlds;
- a participant's authorization may be revoked;
- only one vessel can consume the reserve berth;
- observation has energy and delay costs;
- a shared-source signal cannot legitimately prove independent physical state;
- each participant's service and displacement must remain visible.

The unit of evaluation is not a berth-utilization score. It is a bounded **policy over observations and recourse actions**, with physical feasibility, rights/consent and service outcomes preserved.

## Frozen worlds

Both primary berths are disrupted in every world. There is one reserve berth and two participants, A and B.

| World | Critical participant | Reserve available | A authorized | B authorized |
|---|---|---:|---:|---:|
| a_critical_reserve_ready | A | yes | yes | yes |
| b_critical_reserve_ready | B | yes | yes | yes |
| a_critical_reserve_outage | A | no | yes | yes |
| a_critical_a_consent_revoked | A | yes | no | yes |
| b_critical_b_consent_revoked | B | yes | yes | no |

Each primary berth disruption leaves 30 service_units for each participant without recovery. Assigning the reserve berth to A gives A 80 and B 30; assigning it to B gives A 30 and B 80. The critical participant misses its deadline unless the reserve is assigned to it. The service values and deadline semantics are synthetic, declared model inputs.

## Objective and hard constraints

The objective is lexicographic, not a weighted blend:

1. Minimize additional critical-connection misses relative to the best physically feasible action in each world.
2. Minimize service-unit loss, including explicit observation cost.

Hard constraints are applied before objective comparison:
- reserve capacity can be allocated to at most one participant;
- a reserve assignment is infeasible when the reserve berth is unavailable;
- a participant's reassignment is infeasible when its current authorization is absent or revoked;
- advisory-only or no-observation policies cannot use reserve capacity;
- unknown, potentially-correlated, revoked, stale, incomplete, ambiguous, unmatched or unauthorised evidence cannot qualify an independent observation.

The participant-level outcomes (A units, B units, missed critical participant, authorization disposition and reserve use) are emitted separately. We do not collapse distributional incidence into a hidden fairness weight. The CSV also emits each observation partition's regret separately from each world-level regret, so the decision boundary and aggregate worst-case are independently inspectable.

## Observation alternatives

| Policy | Evidence status | Energy | Delay | Cost |
|---|---|---:|---:|---:|
| no_observation | No new evidence | 0 kWh | 0 ticks | 0 |
| shared_tos_signal | Advisory only; shared gateway | 2 kWh | 0 ticks | 2 |
| independent_reserve_inspection | Independent candidate with unique correspondence and current consent snapshot | 3 kWh | 2 ticks | 7 |
| overbudget_independent_audit | Admissible but over budget | 5 kWh | 3 ticks | 11 |
| other negative controls | Blocked before outcome partitioning | 0 kWh | 0 ticks | NA |

The synthetic conversion is explicit: 1 service_unit per kWh and 2 service_units per acquisition-delay tick. The acquisition budget is 8 service_units.

The shared TOS signal distinguishes A-critical from B-critical reports but shares the TOS/gateway failure path and cannot qualify reserve availability. Therefore it may remain advisory but cannot authorize reserve allocation.

The independent inspection separates all five worlds. Participant authorization is not inferred from the physical inspection: a current Mycelix-style authorization snapshot is separately required. The reserve is assigned:
- to A in the A-critical/ready world;
- to B in the B-critical/ready world;
- to nobody when the reserve is unavailable;
- to B when A's reallocation consent is revoked;
- to A when B's reallocation consent is revoked.

## Hand-calculated known answers

| Policy | Worst additional critical misses | Worst service loss including observation cost |
|---|---:|---:|
| no_observation | 1 | 50 |
| shared_tos_signal | 1 | 52 |
| independent_reserve_inspection | 0 | 7 |

This known-answer result demonstrates an important distinction: the independent inspection is valuable not because it maximizes information gain, but because it enables a feasible, rights-respecting allocation that avoids a critical deadline miss. The shared TOS signal costs 2 service_units without making reserve use admissible. The exact observation costs 7 but reduces the first component of regret from 1 to 0.

These values are from a small finite synthetic fixture. They are not measured port performance, a causal impact estimate, or evidence that Luminous outperforms an installed TOS.

## Independent finite oracle

The production policy solver minimizes worst-case lexicographic regret independently within each observed outcome partition.

The test-only oracle takes a separate route:
- enumerate every mapping from observation partitions to the three possible actions;
- reject mappings that violate reserve availability, participant authorization or observation-mode limits;
- calculate world outcomes using a direct reference formulation;
- compute each policy's worst-case regret;
- select the lexicographically best full policy, including deterministic tie-breaking.

There are 52 set partitions of five worlds. The test suite compares the production solver with exhaustive enumeration over every partition and the full three-action policy space. It also checks 64 deterministically generated five-world sets across all 52 partitions (3,328 generated-world/partition comparisons), in addition to the hand-authored known-answer worlds, and compares the three named observation policies with the independent oracle.

## Current owner boundaries

- Symtropy #7007 / PR #7009: unique physical-event correspondence; this benchmark consumes a synthetic disposition and does not implement matching.
- Symtropy #7037 / PR #7081: stage × dimension dependency separation; this benchmark consumes a synthetic disposition and does not implement dependency closure.
- Symtropy #6971: capture completeness and source/witness profile; future owner adapter.
- CIVDYN-ACTIVE-SENSING-001 #4782 / PR #7166: cost-bearing observation action; this benchmark adds contested multi-vessel recourse rather than a second observation engine.
- MARITIME-REC-001 #1867: bounded recovery policy owner.
- MARITIME-JIT-001 #1858: voyage-arrival/berth/energy coordination.
- Mycelix: participant-authoritative permission, revocation and custody receipts. A synthetic authorization snapshot is not a real authorization.
- Symtropy: deterministic optimization, replay, objective vectors and finite scenario evaluation.

## Industry and research comparators

- **NoisyNet-A3C yard scheduling (Frontiers in Marine Science, 21 September 2026):** reports 23.9% lower yard-crane operating time and 21.7% lower internal-truck waiting time against a terminal dispatch scheme in its 30-day Q3 2024 case study. Its authors distinguish parameter-space exploration noise from operational disturbance robustness and identify external vessel delays, truck congestion and hub-scale complexity as limitations. Reproduce its local yard case before comparing. https://doi.org/10.3389/fmars.2026.1953124
- **Robust BQCYAP (Computers & Industrial Engineering, August 2026):** integrates berth, quay-crane and yard assignment in a two-stage robust model; improved column-and-constraint generation supports exact solution and a C&CG heuristic addresses large cases. This is a serious mathematical scheduling baseline. https://doi.org/10.1016/j.cie.2026.112092
- **Robust berth/yard/shore-power planning (Ocean Engineering, 15 June 2026):** addresses vessel-arrival uncertainty, OPS compatibility and carbon costs using adversarial hierarchical multi-agent SAC; reported 4–13% cost savings are specific to that study's deterministic baseline and test setup. https://doi.org/10.1016/j.oceaneng.2026.125850
- **Integrated berth/crane/OPS and multi-source energy dispatch (Journal of Cleaner Production, 18 August 2026):** models berth, quay crane, yard crane, OPS and energy dispatch; reports up to 19.87% cost reduction in its case studies. https://doi.org/10.1016/j.jclepro.2026.149105
- **Dynamic double-line multi-ship mooring (Computers & Industrial Engineering, October 2026):** jointly considers berth and quay-crane allocation, feeder replacement and direct transshipment with an improved Benders decomposition. https://doi.org/10.1016/j.cie.2026.112236

These are not directly comparable to the finite recourse benchmark. They are baseline owners to reproduce on their own matched problem definitions.

## Standards and integration boundaries

- **DCSA Track & Trace 3.0.0** was published on 30 September 2026 with IoT/reefer events, corrections/withdrawals and intermediary exchange. DCSA says initial access is for members and DCSA+ partners, with public availability planned for H2 2027. Do not assume unrestricted public access. https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility
- **GS1 EPCIS SensorReport** keeps sensor observation time distinct from business eventTime and repository recordTime. Preserve timestamp role semantics when building event adapters. https://ref.gs1.org/epcis/SensorReport
- **NIST SP 800-82 Rev. 4 Initial Public Draft** (21 September 2026) expands OT guidance to maritime vessels, freight rail and IIoT/cloud convergence. It remains a draft; comments are due 30 November 2026. Use it as a pilot design input, not as a final standard. https://csrc.nist.gov/pubs/sp/800/82/r4/ipd

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path research/maritime-scarce-reserve-recovery/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/maritime-scarce-reserve-recovery/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/maritime-scarce-reserve-recovery/Cargo.toml --all-targets
cargo run --quiet --locked --manifest-path research/maritime-scarce-reserve-recovery/Cargo.toml --bin scarce-reserve-recovery
```

## Claim ceiling and next gates

This is synthetic finite-state software research only. It does not qualify a berth, prove a cargo receipt, authorize reallocation, book a resource, command a TOS, change a vessel plan or control OT.

Next gates:
1. Generate many small two-vessel worlds and exhaustively compare production policy choices with the independent oracle.
2. Add a third participant, two reserve assets, different compatibility classes and competing simultaneous disruption pairs.
3. Model non-myopic recourse: reallocation consumes reserve inventory and changes downstream connections.
4. Add outcome-dependent inspection failure and MNAR missingness under common-cause disruption.
5. Add explicit participant displacement, fairness constraints and Pareto reporting without weighted averaging.
6. Consume real versioned read-only owner receipts only after schema, rights and authority reviews.
7. Reproduce NoisyNet-A3C, robust BQCYAP, OPS-aware berth/yard, and multi-ship mooring baselines on matched instances before any SOTA superiority claim.

A simulated plan is not operational authority or physical qualification.
