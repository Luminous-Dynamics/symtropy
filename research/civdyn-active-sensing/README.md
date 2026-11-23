# Maritime active-sensing and contingent-recourse reference v0.1

This is the first executable maritime application case for CIVDYN-ACTIVE-SENSING-001 (#4782). It composes the observation-action boundary with the existing maritime evidence owners and the bounded recovery programme. It is not a new event matcher, dependency graph, generic VOI engine, terminal optimizer, or operational controller.

## Research question

For a small port-call disruption, should the decision-maker:

1. act on current evidence;
2. acquire another observation, paying its energy and delay cost; or
3. abstain because the evidence cannot support the required claim?

The objective is to minimize worst-case downstream service regret after the sensing action's declared cost. No probabilities are assigned to model worlds, no model-count priors are used, and no information-gain score substitutes for decision value.

## Composition and ownership

- Symtropy #7009 / MARITIME-PHYSICAL-EVENT-ASSOCIATION-001: consume a unique physical-event correspondence disposition; this crate does not match events.
- Symtropy #7081 / MARITIME-CHANNEL-DEPENDENCY-BINDING-001: consume a role × stage × dimension dependency disposition; this crate does not discover dependencies or assert independence itself.
- Symtropy #6971: consume capture-completeness and channel-qualification results.
- CIVDYN-DECISION-VOI-001 #4620: the fixed-experiment, prior-free decision-regret framework is the conceptual and protocol owner. This crate does not claim Bayesian EVSI or create a second general experiment selector.
- CIVDYN-ACTIVE-SENSING-001 #4782: owns the cost-bearing sensing-action extension. Energy and acquisition latency reduce service value before recourse is selected.
- MARITIME-REC-001 #1867 and MARITIME-JIT-001 #1858: own the bounded recourse and port-call planning context. This fixture only selects from a tiny predeclared action catalogue; it does not replace either planner.

The evidence receipt strings in this fixture are synthetic identifiers. They are not cryptographic digests, signatures, authenticated receipts or real upstream API objects. The gate enums represent already-computed owner dispositions, not a local reimplementation of those owners.

## Frozen synthetic case

Three hidden worlds are declared:

| World | Primary berth | Reserve option | Clairvoyant service |
|---|---|---|---:|
| primary_ready | Ready | Available | 100 |
| primary_delayed_reserve_ready | Delayed | Available | 90 |
| primary_delayed_reserve_unavailable | Delayed | Unavailable | 20 |

Candidate recourse actions:

| Action | Ready world | Delayed / reserve ready | Delayed / reserve unavailable |
|---|---:|---:|---:|
| maintain_primary | 100 | 20 | 20 |
| use_reserve | 80 | 90 | Infeasible |
| hold | 0 | 0 | 0 |

The no-observation baseline must choose one action feasible in all worlds. Its minimax-regret choice is maintain_primary, with 70 service_units worst-case regret.

### Observation alternatives

| Observation | Evidence ceiling | Energy | Delay | Worst regret | Regret reduction vs no observation |
|---|---|---:|---:|---:|---:|
| no_observation | No new evidence | 0 kWh | 0 ticks | 70 service_units | 0 service_units |
| shared_tos_signal | Advisory only; shared gateway | 1 kWh | 0 ticks | 71 service_units | -1 service_unit |
| independent_field_inspection | Independent observation candidate | 2 kWh | 1 tick | 5 service_units | 65 service_units |
| unknown_dependency_feed | Not admissible | 0 kWh | 0 ticks | NA | NA |
| ambiguous_event_camera | Not admissible | 0 kWh | 0 ticks | NA | NA |
| stale_field_sensor | Not admissible | 0 kWh | 0 ticks | NA | NA |
| overbudget_independent_audit | Above budget | 4 kWh | 2 ticks | NA | NA (12 cost units > 8 budget) |

Conversion is explicit and synthetic: 1 service_unit per kWh and 3 service_units per acquisition-delay tick. The independent inspection therefore costs 2 + 3 = 5 service_units. Its three distinct, qualified outcomes select maintain_primary, use_reserve, and maintain_primary respectively. The reserve action is never admitted in the world where reserve capacity is unavailable.

The shared TOS signal distinguishes a ready primary berth from a missing report, but the two delayed worlds remain in the same observation partition. Its dependency is known to be shared, so it is available only as advisory telemetry and cannot be promoted to independent corroboration. Its 1-service-unit cost makes its worst-case regret 71, worse than taking no observation.

The unknown-dependency, ambiguous-association and stale-evidence controls are blocked before they can partition worlds. The over-budget inspection is not recommended even though its synthetic outcome predictions perfectly discriminate all three worlds.

## Reproduction

From the repository root:

```sh
cargo fmt --manifest-path research/civdyn-active-sensing/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --all-targets
cargo run --quiet --locked --manifest-path research/civdyn-active-sensing/Cargo.toml --bin maritime-active-sensing
```

The executable emits a deterministic CSV receipt. CI runs it twice and compares the output byte-for-byte.


## Current industry and research comparators

The benchmark is intentionally not a terminal operating system or a replacement for terminal dispatch. It should eventually be compared on matched subproblems and then on end-to-end delivered service.

- **NoisyNet-A3C yard scheduling (Frontiers in Marine Science, 2026):** reports 23.9% lower yard-crane operating time and 21.7% lower internal-truck waiting time against the Haitian Terminal dispatch scheme, using 30 consecutive days of Q3 2024 operational logs. The paper explicitly says its parameter noise improves training exploration rather than modeling operational disturbances; it identifies vessel berthing delays, internal-truck congestion and scaling to large hubs as limitations. Reproduce its single-vessel-shift yard problem before comparing. https://doi.org/10.3389/fmars.2026.1953124
- **Robust berth/quay-crane/yard allocation (Computers & Industrial Engineering, 2026):** two-stage robust optimization with column-and-constraint generation and a large-instance heuristic under uncertain vessel arrival and container volumes. https://doi.org/10.1016/j.cie.2026.112092
- **Robust berth/yard/shore-power allocation (Ocean Engineering, 2026):** integrates berth and yard planning with OPS compatibility under uncertain arrivals. It is a relevant baseline for energy-aware pre-arrival planning, not evidence that physical delivery or source independence is assured. https://doi.org/10.1016/j.oceaneng.2026.125850
- **Multi-ship block sharing (Transportation Research Part E, 2026):** couples time-varying quay-crane assignment and yard-crane capacity under external-truck arrival and yard-health uncertainty. https://doi.org/10.1016/j.tre.2026.105314

These studies are not directly comparable to this three-world active-sensing fixture. They define the next baseline ladder: reproduce the published local dispatch problem; add standard A3C and reactive scheduling; add rolling-horizon ALNS/strong heuristics; include two-stage robust optimization/column-and-constraint generation; then compare against the installed TOS baseline when rights permit. Only after matched local replication should we compare joint port-call recourse and end-to-end service.

## Standards and measurement boundaries

- **DCSA Track & Trace 3.0.0** was published on 30 September 2026 and adds IoT and reefer events, corrections/withdrawals and intermediary exchange. DCSA states that initial access is for members and DCSA+ partners, with public release planned for H2 2027. Do not assume general public availability as of this benchmark date. https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility
- **GS1 EPCIS SensorReport** distinguishes sensor observation time from the business-step event time. If the sensor time field is present, it must be earlier than or equal to eventTime; eventTime describes completion of the business step, not when a measured condition held. Repository recordTime is bookkeeping, not physical event time. https://ref.gs1.org/epcis/SensorReport
- **World Bank / S&P Global CPPI 2025** measures port-level vessel time in port, including waiting and berth time. Its methodology says the index is port-level, not terminal-level, and excludes time after the vessel has left berth and before it exits port limits. It is a useful external diagnostic, not a cargo-level custody or receiver-delivery benchmark. https://www.worldbank.org/en/topic/transport/publication/cppi and https://thedocs.worldbank.org/en/doc/855fe5e7123d85b05d4e84c95b8fd1e2-0400012026/original/Methodology-Note.pdf
- **NIST SP 800-82 Rev. 4 Initial Public Draft** (21 September 2026) explicitly expands OT guidance to maritime vessels, freight rail and IIoT/cloud convergence. The comment deadline is 30 November 2026; it remains a draft and must not be described as a final standard. https://csrc.nist.gov/pubs/sp/800/82/r4/ipd

## Benchmark ladder beyond the toy case

| Stage | Gate | Proposed target |
|---|---|---|
| P0 | Tiny finite known-truth cases and independent exhaustive oracle | 100% exact oracle agreement; zero false unique/complete/separated/eligible claims |
| P1 | DCSA/EPCIS and owner-schema adapters | No silent semantic, timestamp, revision, rights or dependency loss |
| P2 | Independent site/inventory adjudication | At least 3,000 role × stage × dimension decisions, clustered and reported by site, operator and common-cause root |
| P3 | Rights-cleared read-only shadow mode | Measure observation recall, correction lag, common-cause discovery, abstention, and p50/p95/p99 latency |
| P4 | Matched intervention | Research target: at least 10% lower backlog area or p90 delay, with no safety, fairness, service, energy or emissions regression |
| P5 | Independent physical delivery adjudication | Reconcile custody, physical handling, receiver receipt, cargo quantity, time and resource use |

Targets are hypotheses for the research programme, not demonstrated results or industry standards. The 3,000-decision target must not be satisfied by repeated records from one gateway or one operator. Statistical intervals must account for clustering and held-out sites/time periods. The P4 target is only meaningful against matched operating conditions and a predeclared baseline; a simulator win or a TOS dispatch comparison alone is insufficient.

Never combine safety, rights, consent, cargo conservation or hard capacity violations into a weighted score that can be compensated by throughput. Report service, delay, energy, emissions, cost, fairness, abstention, feasibility and computational burden separately.

## Claim ceiling

This is synthetic finite-state software evidence only. It demonstrates that, in this frozen toy case, a cost-bearing independent inspection can reduce worst-case decision regret from 70 to 5 service_units while shared, stale, ambiguous, unknown and over-budget observations are not misrepresented as independent evidence or selected.

It does not establish field efficacy, statistical superiority, causal identification, real port performance, operational safety, delivery of cargo, or permission to execute a recovery action. A unique event match is not a cargo delivery proof. An independent observation is not a custody receipt. A selected recovery action is only a proposal over synthetic, predeclared alternatives.

## Next qualification gates

1. Replace synthetic gate values with versioned read-only receipts from the owners of #6971, #7007 and #7037, without forking their logic.
2. Expand to missing-not-at-random observations, shared outage causes, delayed/failed inspections, revoked permissions and observation actions that alter physical state beyond a scalar service penalty.
3. Add a second contingent disruption and a scarce reserve resource; test simultaneous claims and participant-level displacement under #1867.
4. Compare no observation, shared-source telemetry, independent inspection, fixed rules and clairvoyant evaluator-only oracle on the same frozen worlds.
5. Add generated small cases and an independent exhaustive oracle; do not promote a scenario family based only on hand-authored fixtures.
6. Only after source rights, shadow-mode permissions and independent adjudication are established should real terminal/receiver outcomes be introduced.

Simulation PASS is not physical qualification or authority to operate.
