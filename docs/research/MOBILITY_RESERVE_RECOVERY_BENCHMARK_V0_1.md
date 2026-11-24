# Mobility Reserve Recovery and Repeated-Shock Benchmark v0.1

**Programme:** [MOBILITY-ENERGY-RESILIENCE-001 #6707](https://github.com/Luminous-Dynamics/symtropy/issues/6707)  
**Recovery implementation issue:** [#6808](https://github.com/Luminous-Dynamics/symtropy/issues/6808)  
**Predecessor assurance gate:** [#6802 / PR #6803](https://github.com/Luminous-Dynamics/symtropy/pull/6803)  
**Branch:** \`research/mobility-reserve-recovery-v0-2\`  
**Cut-off:** 22 November 2026  
**Status:** source-level v0.2 evaluator, strict schema, synthetic repeated-shock fixture, CLI and workflow integration committed; Rust formatting/compile/test execution remains pending on this branch.

## 1. The question v0.1 does not answer

The v0.1 flexibility verifier checks that a supplied per-resource discharge allocation fits each asset's reserve-adjusted energy and power budgets during one offer window. It now rejects a schedule where one asset's unused energy or power masks another asset's shortfall, and its 625-profile exact-reference test passed in CI.

It does **not** establish that the depot can replenish the withdrawn energy before the next mission, or that it can survive a second event while recovering. Those are operationally important failure modes. A depot may complete a grid event and restore its headline service rate while its reserve remains depleted, leaving it fragile to the next disturbance.

Recovery therefore needs its own chronological state proof.

## 2. State-transition contract

For each exact resource \(r\), scenario \(s\), and slot \(t\), carry a conservative stored-energy lower bound \(E_{r,s,t}\). Charging energy is rounded down; energy withdrawn is rounded up.

\[
E_{r,s,t+1}
=
E_{r,s,t}
+
E^{charge,lb}_{r,s,t}
-
E^{mission,ub}_{r,s,t}
-
E^{disturbance,ub}_{r,s,t}
-
E^{grid,ub}_{r,s,t}
\]

For a charge input-power bound \(P^{ch}_{r,t}\), duration \(\Delta t\), and qualified charge-efficiency lower bound \(\eta^{ch,lb}_r\) expressed in permille:

\[
E^{charge,lb}_{r,t}
=
\left\lfloor
\frac{P^{ch}_{r,t}\Delta t\eta^{ch,lb}_r}{3{,}600{,}000}
\right\rfloor
\]

For delivered discharge-power \(P^{grid}_{r,t}\) and qualified discharge-efficiency lower bound \(\eta^{dis,lb}_r\):

\[
E^{grid,ub}_{r,t}
=
\left\lceil
\frac{P^{grid}_{r,t}\Delta t \cdot 1000}
{3600\eta^{dis,lb}_r}
\right\rceil
\]

The implementation must use checked integer arithmetic; it must never let float rounding create energy. These simplified equations are only valid when the supplied efficiencies refer to the exact power/energy boundary being modeled. Battery-terminal, DC bus and PCC measurements cannot be interchanged.

At every slot boundary:

\[
R^{mission}_{r,s,t}
+
R^{inventory}_{r,s,t}
+
R^{safety}_{r,s,t}
+
R^{uncertainty}_{r,s,t}
\le E_{r,s,t}
\le E^{capacity,lb}_r
\]

A future mission is a dated obligation, not a horizon-average reserve. If mission \(m\) is due at slot \(d_m\), then after its modeled energy use:

\[
E_{r,s,d_m+1}
\ge
E^{post-mission buffer}_{r,m}
+
R^{safety}_{r,s,d_m+1}
\]

The minimum reserve-restoration deadline is an explicit input. “Grid event ended”, “charger session started” or “aircraft is available” is not equivalent to reserve restoration.

## 2A. Current executable prototype and frozen known answer

The initial implementation lives in `tools/ev-passenger-cost-frontier/src/mobility_recovery_envelope.rs`. It is intentionally bounded to at most four resources, four scenarios and sixteen slots. Its current fixture uses two resources and eight 15-minute slots, with two chargers/channels and two demand/weather scenarios.

The recurrence applies withdrawals (mission, disturbance and grid delivery) before crediting that slot's planned recovery charge. Charge energy is rounded down and discharge cost is rounded up using checked integer arithmetic. The initial stored-energy state must already satisfy the starting reserve floor; charging in slot zero cannot backfill an initial reserve breach. The end-of-slot state is then checked against the per-slot reserve floor and the energy capacity. A resource that underflows or exceeds capacity is marked infeasible even if later slots would restore it.

In the harder `second-shock-before-recovery` fixture, the independently enumerated state traces for slots 0–5 are:

| Resource | Energy after slots 0–5 (Wh) | Slot-5 target | Final margin |
| --- | --- | ---: | ---: |
| ess-01 | 838, 676, 901, 1,126, 926, 1,151 | 1,100 Wh | +51 Wh |
| ess-02 | 724, 638, 818, 998, 848, 960 | 950 Wh | +10 Wh |

The PCC import trajectory is [1,900, 1,900, 4,300, 4,300, 2,500, 4,000, 2,500, 2,500] W, below the synthetic 6,500 W limit. These are static arithmetic checks against a synthetic fixture, not a Rust execution result or a field guarantee. The branch includes 11 focused Rust test definitions and an existing workflow invocation; the workflow must pass at the exact PR head before formatting, compilation or tests can be claimed.

## 3. Coupled constraints

Recovery must not be a second independent queue or meter. It composes existing evidence owners:

| System | Required input |
| --- | --- |
| #6700 | confirmed resource state, occupancy, reconciliation cut-off and consume-once operation outcome |
| #6705 | exact charger/channel compatibility, reservation interval and queue schedule |
| #6674 | qualified inventory and aircraft/mission allocation |
| #6083 / #6177 | mission energy and deadline obligations |
| #5832 / #5241 | site net load, PCC import envelope and metering boundary |
| #6308 | exact pack, BMS, firmware, charger/inverter and adapter configuration qualification |
| Mycelix | resource identity, custody, evidence lineage and trust |
| Local electrical protection | independent protection and switching; never controlled by this verifier |

The recovery trajectory must additionally check:

- no resource charges and discharges simultaneously unless that exact operating mode has been qualified;
- the same charger channel is not reserved by two resources in one slot;
- resource charge power is bounded by the exact charger channel and pack charge envelope;
- total recovery charge plus existing mobility charging and declared base load stays inside the PCC import limit;
- charge-queue reservation is not credited as stored energy before a physically confirmed operation;
- site outages, charger outages and unknown physical outcomes are not silently replaced by nominal charging;
- the same battery's future mission and grid-service obligations do not double-claim its energy.

## 4. Repeated-shock benchmark

Each recovery scenario is a chronological timeline, not a bag of independent reserve numbers. The canonical hard case is:

1. Start with independently qualified initial energy and inventory.
2. Apply event 1: grid-service request or contingency discharge.
3. Enter a constrained recovery period with realistic charger-channel and PCC headroom.
4. Apply event 2 before all assets have recovered; include common-cause conditions such as low renewable output plus charger or feeder derating.
5. Enforce the next mission's exact pack/energy/power deadline.
6. Continue until each required resource reaches its full reserve-restoration target—or the evaluator explicitly abstains.

The evaluator reports the first failed asset, slot and obligation; the minimum reserve margin; the time reserve was restored; the energy remaining before the second event; and whether the second event and next mission both remain feasible.

A single “survived one event” bit is not enough. Publish recovery trajectories and the second-shock timing window.

## 5. Independent baseline tournament

The first comparison should reproduce the relevant road-EV methods on their own source testbeds and only then create matched UAV/carrier scenarios:

- fixed charge recovery / no grid participation;
- earliest-deadline-first recovery;
- greedy lowest-tariff recovery;
- deterministic MILP;
- rolling-horizon MPC;
- two-stage robust optimization with column-and-constraint generation;
- Wasserstein or other distributionally robust optimization;
- offline DR-SAC policy proposals;
- each optimizer behind the same independent resource-level reserve and recovery verifier.

The 2026 DR-SAC heavy-truck station study is a particularly useful counterexample: it reports 78.9% peak shaving for one selected medium-demand peak window, but its all-medium-day figure is 67.7% ± 20.5%; in its high-demand scenario it reports 5.3 violating periods/day. SAC has fewer violations (2.9/day) in that high-demand test, while DDPG reaches 86.7% peak shaving with 21.7 violating periods/day. The policy trade-off is precisely why average reward or peak shaving cannot stand in for a per-asset recovery contract. These are study-specific road-EV results, not UAV performance claims. [Du & Zhu, 2026](https://doi.org/10.3390/app162211346)

The 2026 battery-swap cluster literature also demonstrates multi-timescale scheduling: day-ahead commitments, 15-minute rolling charge/discharge schedules and finer-grained station recommendations. Reproduce these baselines faithfully, but add explicit identity/configuration-bound reserves and a repeated-event recovery horizon before comparing. [Chen et al., 2026](https://doi.org/10.3390/electronics15112444)

## 6. Metrics and target ladder

Targets are research goals, not achievements:

| Metric | Target / reporting rule |
| --- | --- |
| Hard mission-floor violation | 0 across every admitted scenario |
| Recovery deadline | 100% restored by deadline or explicit abstention |
| Second-shock viability | 100% feasibility for the frozen hard-case suite; publish denominator |
| Independent exact oracle | 100% agreement on the bounded small-instance family |
| False acceptance | 0 for stale, replayed, mismatched or unknown hard-boundary inputs |
| Resource energy arithmetic | checked integer arithmetic; no negative state and no energy created by rounding |
| Flexibility after recovery obligation | publish deliverable kW/kWh only after reserving the next mission and the recovery trajectory |
| Recovery time | p50/p95/p99 by resource class and scenario |
| Queue / infrastructure | p95/p99 queue wait, channel occupancy, charger outage sensitivity and PCC headroom |
| Economics | lifecycle cost per successful on-time mission; verified grid-service kWh reported separately |
| Optimization | optimality gap, timeout rate, regret and runtime p50/p95/p99 on matched frozen cases |

Larger-case targets: mean regret ≤1% and p95 ≤5% against an exact/MILP reference where tractable. An infeasible or timed-out case is not dropped from the denominator.

## 7. Adversarial negative controls

- Event 1 is feasible but the reserve cannot recover by the next mission.
- The pack reaches the next mission with less than its qualified mission energy.
- Event 2 occurs before full restoration and violates the reserve floor.
- A stale charger-queue receipt is presented as a completed charge.
- A timed-out physical charging operation is retried before #6700 reconciliation.
- One resource is recharged in the plan but its channel is double-booked.
- Charger-side input watts are incorrectly credited as pack-terminal stored watt-hours without efficiency.
- Charge efficiency is unknown, stale, or attached to another configuration.
- SOC, temperature or SOH derating is unknown but nominal capability is assumed.
- PCC headroom is insufficient after existing charging and site load.
- A resource from another site is included in a pooled energy balance.
- An event/mission is counted twice or a reserve receipt is replayed.
- Arithmetic overflows, time horizon is non-contiguous, or input order changes the digest.
- The evaluator calls the depot “recovered” before every required asset reserve is restored.

## 8. Phasing

**v0.2a — exact energy recurrence.** Conservative integer energy state, mission floors, separate reserve categories, explicit charge/discharge efficiency bounds and one repeated-shock synthetic fixture.

**v0.2b — infrastructure coupling.** Consume #6705 channel reservations and #5832/#5241 PCC headroom; reject duplicate channels and unknown physical charge outcomes.

**v0.2c — exact oracle.** Exhaustively enumerate small resource/slot cases and compare against an independently implemented recurrence.

**v0.3 — thermal/SOH and robust scenarios.** Add only configuration-bound and independently qualified derating data. Introduce correlated, unseen and common-cause event families.

**v0.4 — measured field qualification.** Compare predicted SOC/energy with independently measured pack-terminal and PCC energy; report calibration error and degradation. No simulator-only pass can substitute for field evidence.

## 9. Claim boundary

A green synthetic recovery result means only that the supplied finite state trajectory satisfies the supplied conservative energy recurrence, resource-level reserves, charger constraints and PCC constraints. It does not certify electrochemical safety, thermal stability, battery lifetime, interconnection, grid-export rights, protection, flight safety or commercial delivery. The evaluator is a proposal qualification screen; it cannot operate a charger, inverter, BMS, contactor or aircraft.
