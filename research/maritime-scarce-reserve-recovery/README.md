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
| a_critical_reserve_ready_sensor_only_fault | A | yes | yes | yes |

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

The shared TOS signal has an explicit **outcome-dependent missingness control**. Its observable labels are:
- A-priority in the first and fourth worlds;
- B-priority in the second and fifth worlds;
- MISSING in both the reserve-plus-gateway common-cause outage world and the sensor-only-failure world where reserve capacity is actually healthy.

The evaluator separately retains the hidden cause of each missing observation. The policy sees only the shared MISSING token and must not infer which hidden cause occurred. This is a deterministic finite structural test of non-ignorable/missing-not-at-random observation behavior; it does not estimate statistical missingness probabilities, calibrate a stochastic likelihood or establish field missingness rates. The hidden cause is deliberately not part of the `World` type passed into the policy solver; it appears only in the evaluator's adjudication receipt.

The independent inspection separates all six worlds. Participant authorization is not inferred from the physical inspection: a current Mycelix-style authorization snapshot is separately required. The reserve is assigned:
- to A in the A-critical/ready world;
- to B in the B-critical/ready world;
- to nobody when the reserve is unavailable;
- to B when A's reallocation consent is revoked;
- to A when B's reallocation consent is revoked;
- to A in the sixth world, where reserve capacity is healthy but the shared telemetry sensor alone has failed.

## Hand-calculated known answers

| Policy | Worst additional critical misses | Worst service loss including observation cost |
|---|---:|---:|
| no_observation | 1 | 50 |
| shared_tos_signal | 1 | 52 |
| independent_reserve_inspection | 0 | 7 |

This known-answer result demonstrates an important distinction: the independent inspection is valuable not because it maximizes information gain, but because it enables the best feasible, rights-respecting allocation in each world. It does **not** mean that no critical deadlines are missed in absolute terms. Three worlds still have an absolute critical miss: the reserve-outage world, the A-consent-revoked world, and the B-consent-revoked world. In each, no consent- and capacity-feasible action can serve that world’s critical participant; the inspection does not add a miss relative to the per-world feasible oracle. The world-level CSV receipt preserves the participant who misses service. The shared TOS signal costs 2 service_units without making reserve use admissible. The exact observation costs 7 and reduces the worst-case **additional** missed deadlines from 1 to 0.

These values are from a small finite synthetic fixture. They are not measured port performance, a causal impact estimate, or evidence that Luminous outperforms an installed TOS.

## Independent finite oracle

The production policy solver minimizes worst-case lexicographic regret independently within each observed outcome partition.

The test-only oracle takes a separate route:
- enumerate every mapping from observation partitions to the three possible actions;
- reject mappings that violate reserve availability, participant authorization or observation-mode limits;
- calculate world outcomes using a direct reference formulation;
- compute each policy's worst-case regret;
- select the lexicographically best full policy, including deterministic tie-breaking.

There are 203 set partitions of six worlds. The test suite compares the production solver with exhaustive enumeration over every partition and the full three-action policy space. It also checks 64 deterministically generated six-world sets across all 203 partitions (12,992 generated-world/partition comparisons), in addition to the hand-authored known-answer worlds, and compares the three named observation policies with the independent oracle.

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

## Faithful reproduction contract for the yard-scheduling SOTA

The 2026 NoisyNet-A3C paper is a useful local-operation baseline, but its instance counts require care:
- Section 5.1 describes 10 simulation instance sets (I1–I10) and Table 4 lists 10 configurations.
- Table 6 and Figures 5–6 label results I1–I12; the prose describes ten test cases, while later analysis refers to I1–I12.
- The paper says CPLEX 20.1 obtains certified optima for the small cases I1–I6; I7–I12 do not have guaranteed CPLEX reference solutions because the solver exceeds its configured time limit / overflows.
- The method is trained and compared across 30 independent runs in parts of the simulation analysis, and the paper reports a 30-day Q3 2024 operational-data case study for Haitian Terminal.
- The paper explicitly distinguishes NoisyNet parameter noise (training exploration) from environmental disturbance modelling. It states that the method alone does not guarantee robustness to vessel berthing delays or internal-truck congestion, and that the model is a single-vessel berthing-shift formulation rather than a hub-scale hundreds-of-cranes/thousands-of-tasks problem.

Reproduction protocol before any comparison:
1. Resolve the I1–I10 vs I1–I12 discrepancy against the published supplementary materials and authors where necessary; preserve every reported row and mark unresolved correspondence rather than silently dropping cases.
2. Rebuild the MILP objective, vessel/quay-crane precedence, yard-crane path and safety-distance constraints, internal-truck waiting model, and all units.
3. Reproduce CPLEX 20.1 results on the small certified-optimum instances; report optimality gaps only where an actual bound exists. For larger instances, compare against best-known feasible values and runtime, not a fictional optimality gap.
4. Reproduce Actor-Critic, A3C and NoisyNet-A3C with the paper's backbone, hyperparameter search space, random-seed protocol, parallelism and stopping criteria. Publish mean, median, dispersion and confidence intervals, not a single best run.
5. Replicate the 30-day Q3 2024 day-by-day baseline only if the underlying logs and data rights are available. Otherwise label the result as a paper reproduction, not a new operational replication.
6. Add an independent disturbance suite: vessel arrival delay, quay-crane failure, internal-truck congestion, yard health loss, reefer/energy constraint, common-cause telemetry outage, reserve berth contention and participant-rights revocation.
7. Report original paper KPIs separately from end-to-end service: yard crane operating time, internal-truck waiting, vessel delay, cargo service, receiver deadline misses, reserve displacement, energy/emissions and evidence false-admission rates.

The paper's DOI is https://doi.org/10.3389/fmars.2026.1953124. Specific results (23.9% and 21.7%) are author-reported in that paper's 30-day terminal case study, not transferable targets for this synthetic recourse benchmark.

## Commercial positioning and integration comparators

This programme should complement installed TOS and planning platforms rather than pretend to replace their operational control role.

- **Kaleris N4** presents itself as the terminal operational foundation integrating vessel, yard, rail, gate and control-room operations, with advanced optimization for RTGs, terminal trucks and yard intelligence. Vendor overview: https://kaleris.com/terminal-operating-system/kaleris-n4-tos/
- **Tideworks Mainsail 10 + Spinnaker** went live at POMTOC at PortMiami in March 2026. Tideworks describes a cloud TOS, planning system and standardized API integration framework. Deployment reference: https://tideworks.com/pomtoc-accelerates-terminal-modernization-to-support-regional-trade-growth-with-tideworks-cloud-tos/
- **Portchain Quay** positions itself as a dedicated terminal berth-planning tool, including crane/labour resources, maintenance schedules, vessel tracking, scenario planning and real-time collaboration. Product reference: https://portchain.com/portchain-quay
- **project44 Ocean Visibility** offers carrier/forwarder connections, multimodal shipment stitching, predicted ETAs and exception handling. Its 150+ connections, 98% global ocean-volume coverage and 350K+ daily container figures are vendor-reported coverage claims, not independent measurements of event truth or physical receipt. Product reference: https://www.project44.com/platform/visibility/ocean/

Integration strategy:
1. Consume TOS plans and actual dispatch receipts as owner-scoped operational inputs.
2. Consume berth and carrier visibility feeds while preserving source, timestamp role, revision, correction and dependency lineage.
3. Qualify evidence independently before it changes resource eligibility or a recovery policy.
4. Present bounded recourse recommendations and participant displacement to the responsible operator.
5. Keep execution and commitments with the authorized TOS/operator/participant systems.
6. Independently reconcile physical custody, delivery, service and resource use after execution.

The differentiator is cross-system evidence assurance and network-level decision quality, not replacing each vendor's native execution system.

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
