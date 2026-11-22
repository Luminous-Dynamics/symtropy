# Mobility-First Grid Flexibility Envelope Benchmark v0.1

**Programme:** [MOBILITY-ENERGY-RESILIENCE-001 #6707](https://github.com/Luminous-Dynamics/symtropy/issues/6707)  
**Executable child:** [MOBILITY-FLEX-ENVELOPE-001 #6802](https://github.com/Luminous-Dynamics/symtropy/issues/6802)  
**Implementation branch:** \`research/mobility-flex-envelope-v0-1\`  
**Research cut-off:** 22 November 2026  
**Status:** source-level implementation and synthetic known-answer fixture committed; Rust execution, rustfmt, Clippy and CI remain unverified until an exact-head run completes.

## 1. The central shift

The programme should not sell a battery's nominal energy twice. It should sell only a **time-indexed, scenario-qualified flexibility envelope** after mission obligations have been protected.

A single positive residual-energy number is necessary but not sufficient. A site can have 2 kWh left in aggregate and still be unable to deliver the required 3 kW in a particular 15-minute interval. Conversely, every interval can be below its power ceiling while the sum of all intervals consumes more energy than the portfolio can deliver. Site import/export limits can invalidate an otherwise feasible storage trajectory.

The new decision object is a candidate power trajectory \(p_t\) constrained by a single horizon energy budget, time-slot power ceilings, point-of-common-coupling (PCC) limits and every admitted reserve scenario.

For resource \(r\), scenario \(s\), and interval \(t\):

\[
E^{flex}_{r,s} =
\underline{E}_r -
E^{mission}_{r,s} -
E^{inventory}_{r,s} -
E^{safety}_{r,s} -
E^{uncertainty}_{r,s}
\]

\[
P^{flex}_{r,s,t} =
\underline{P}_r -
P^{mission}_{r,s,t} -
P^{inventory}_{r,s,t} -
P^{safety}_{r,s,t} -
P^{uncertainty}_{r,s,t}
\]

The candidate must meet both:

\[
\sum_t \left\lceil \frac{p_t\Delta t}{3600} \right\rceil
\leq
\sum_r E^{flex}_{r,s}
\quad \forall s
\]

\[
p_t \leq \sum_r P^{flex}_{r,s,t}
\quad \forall s,t
\]

and the site-meter constraint:

\[
-P^{export}_{t}
\leq
P^{base}_{t}+P^{charge}_{t}-p_t
\leq
P^{import}_{t}
\]

Export additionally requires an explicit permission receipt. Without that receipt, the model cannot infer reverse-flow permission from a positive storage-energy balance.

This is a conservative finite-scenario envelope, not a proof about unknown scenarios or a substitute for AC power-flow, protection or commissioning studies.

## 2. Architecture and ownership

This slice deliberately composes existing authorities rather than creating a second ledger:

| Owner | Consumed responsibility |
| --- | --- |
| #6674 | qualified pack inventory and mobility allocation |
| #6700 | confirmed state transitions, unknown-outcome reconciliation and resource occupancy |
| #6705 | exact charging queue and scheduled charging load |
| #6707 | portfolio, reserve policy and cyber-physical scenarios |
| #5832 / #5241 | site/PCC metering, energy-source and import/export accounting |
| #6083 / #6177 | mission, sortie, aircraft allocation and independent permission checks |
| #6308 | pack, charger, carrier and interface configuration qualification |
| Mycelix | identity, custody, lineage and trust |
| Local electrical equipment and qualified authorities | physical interconnection, protection, switching and authorization |

The verifier requires the storage resource to be bound to its own configuration and to the exact site configuration. Reserve buckets have separate receipt IDs so the same reserve receipt cannot be entered twice within one scenario. Alternative scenarios are evaluated independently; receipt IDs may recur in mutually exclusive scenarios because those are alternate representations of the same obligation, not simultaneous commitments.

An aircraft propulsion pack is not an energy-storage asset merely because it is idle. v0.1 rejects the propulsion-pack resource class. A later bidirectional-storage class requires independent interface qualification and the exact site connection evidence.

## 3. What the executable v0.1 does

The Rust evaluator at \`tools/ev-passenger-cost-frontier/src/mobility_flexibility_envelope.rs\`:

- admits at most four storage resources, four reserve scenarios and eight slots;
- requires time-bound site, allocation, charging-queue, meter, storage energy, storage power, availability, interface and site-connection evidence;
- checks exact resource/site/configuration binding and evidence validity through the full horizon;
- binds each mission, inventory, safety/outage and uncertainty reserve value to its own resource/configuration-bound receipt ID and evidence envelope;
- binds each per-slot non-charging load to the site-meter receipt and each per-slot charging load to the exact charging-queue report reference;
- checks unique resource IDs, scenario IDs, reserve receipt IDs and contiguous slot indices;
- computes resource-specific residual energy and residual power for each scenario;
- evaluates one caller-supplied discharge profile against cumulative energy, per-slot power and PCC import/export constraints;
- emits reason-coded findings and deterministic output ordering;
- includes an independent arithmetic reference used to compare 625 four-slot candidate power profiles against the implementation;
- never grants dispatch or export authority.

The 625-profile comparison and 253/372 split are test definitions only until the current PR head executes them.

The fixture deliberately uses synthetic evidence. Its two stationary storage descriptors sum to 4,000 Wh and 5,500 W of lower-bound capability. The most demanding reserve scenario leaves a 2,000 Wh horizon budget. The proposed 15-minute profile consumes 1,875 Wh, leaving 125 Wh of modelled energy margin. Its requested power remains inside the worst-case per-slot power envelope, and the PCC profile remains an import-reduction case with no export permission.

These numbers are a synthetic known answer, not observed storage capability, metered site energy, measured charger efficiency or a commercial operating result.

Run:

\`\`\`bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin mobility_flexibility_envelope_screen -- \\
  tools/ev-passenger-cost-frontier/fixtures/mobility_flexibility_envelope_v0_1.json
\`\`\`

Input contract: \`tools/ev-passenger-cost-frontier/schemas/mobility_flexibility_envelope_v0_1.schema.json\`.

### v0.1 limitations that remain explicit

- This is a feasibility verifier for one proposed profile, not an optimizer.
- Its power aggregation assumes that accepted resources are connected to the same evidenced site/PCC; no feeder topology or full AC power-flow is solved.
- There is no separate PV-generation model in v0.1. The supplied base-site load must be the declared net non-charging load at the PCC after any onsite generation already credited by the source owner.
- The finite reserve scenarios do not provide a statistical or distribution-free guarantee for unrepresented events.
- It does not model electrochemistry, SOC trajectories, temperature dynamics, aging, conversion losses or charge curves.
- It does not yet prove post-dispatch recharge/recovery before the next mission. That is a v0.2 requirement; a passing v0.1 profile must not be described as recovery-qualified.
- Evidence references and producer/method strings are declarative. The prototype does not cryptographically authenticate them.
- The report's \`candidate_model_feasible\` is not permission to energize, charge, discharge, export, island, black-start, fly or return a pack to service.

## 4. Literature and SOTA comparator refresh

The latest directly relevant work spans robust optimization, distributionally robust reinforcement learning and cyber-physical resilience. These studies are primarily road-EV and distribution-grid studies. Their mathematical methods are valuable comparators, but their battery duty cycles and grid assumptions must not be silently transferred to aircraft packs.

| Research line | Published comparator | What it contributes | What the Luminous benchmark must add |
| --- | --- | --- | --- |
| Robust VPP + swapping reserve | Gao et al. (2026), two-stage robust VPP scheduling for short-term operating reserve with central charging, distributed swap depots and transport trucks. [DOI](https://doi.org/10.1016/j.ijepes.2026.111598) | Explicit reserve calling and nested column/constraint generation | Bind each offered MW to exact resource, mission reserve, time window and site evidence |
| PV/BSS distributional robustness | Kim et al. (28 Feb 2026), data-driven DRO for PV-enabled swap stations, Volt/VAR, B2B exchange and demand response on an IEEE 33-node distribution system. [DOI](https://doi.org/10.3390/en19051223) | Joint renewable uncertainty, distribution constraints and swapping inventory | Separate a model-feasible setpoint from independently qualified protection and interface authority |
| Multi-scale robust sizing | Wang et al. (30 Mar 2026), two-stage robust PV-storage station sizing and intraday dispatch using multi-bandwidth uncertainty and column-and-constraint generation. [DOI](https://doi.org/10.1016/j.est.2026.120628) | Capacity planning linked to intraday dispatch | Add serial/configuration-level reserve provenance and mission-priority non-encroachment |
| Aggregated swapping/grid regulation | Chen et al. (2 Jun 2026), day-ahead cluster bidding, intraday tracking and rolling station recommendations. [DOI](https://doi.org/10.3390/electronics15112444) | Multi-timescale coordination of swapping and grid regulation | One shared reserve snapshot and evidence cut-off across all time horizons |
| Cyber-physical resilience | Ghelichpour et al. (1 Nov 2026), real-time grid planning with BSS, LSTM proposals, fuzzy Monte Carlo, communication delays, false-data attacks and grid reconfiguration. [DOI](https://doi.org/10.1016/j.renene.2026.126219) | Cyber disturbance and grid resilience in the scheduling formulation | A separate deterministic reserve shield; forecasts and aggregators remain non-authoritative |
| Distributionally robust deep RL | Du & Zhu (16 Nov 2026), offline DR-SAC heavy-truck BSS policy with KL ambiguity set, VAE transition model and SOC-level charge-count actions. [DOI](https://doi.org/10.3390/app162211346) | Current learned-policy baseline: 3.7% relative profit variation and 78.9% peak shaving in one selected configuration; high-demand case still has 5.3 violating periods/day | Preserve profit/peak-shaving comparison, but require a separate reserve gate; count hard violations and recovery failures, not just mean reward |
| AC power-flow coupled scheduling | Yuvaraj et al. (Nov 2026), strategic management of renewable-integrated charging and swapping under uncertainty and full AC power-flow constraints. [DOI](https://doi.org/10.1016/j.esr.2026.102377) | More realistic network feasibility than a site-only energy balance | Add locational feeder constraints and validated network models in a later phase |

### Commercial autonomous-dock comparators (manufacturer specifications)

| Product | Published operating measure | What it is—and is not |
| --- | --- | --- |
| DJI Dock 4 | Battery swap <25 s; landing-to-next-takeoff turnaround <45 s; up to 3 stored batteries; 2 parallel charging channels; 27 min from 15% to 95% charge | DJI states the swap/turnaround figures are from controlled tests and for reference. This is a strong turnaround baseline, not evidence of network-wide on-time service or grid flexibility. [Manufacturer specifications](https://enterprise.dji.com/dock-4/specs) |
| Hextronics Atlas | <2 min swap; 8 sequentially smart-charged battery slots; supplier lists >99.99% battery-swapping reliability | A useful high-inventory comparison. Its reliability number is supplier-published; do not convert it into an independently measured fleet failure rate without denominator, exposure and test-method evidence. [Supplier spec sheet](https://docs.hextronics.cloud/product/spec-sheet-atlas.pdf) |
| Skydio Dock for X10 | Airborne in 20 s | This is launch time, not battery swap time. Keep it in a launch-latency category rather than a swap-speed leaderboard. [Manufacturer technical specifications](https://www.skydio.com/dock/technical-specs) |

The product comparison demonstrates why one headline turnaround number is insufficient. Keep at least five denominators separate: mechanical exchange success, landing-to-launch time, charge restoration time, aircraft/pack network availability, and successful on-time mission delivery. Add grid flexibility as a sixth, independently metered service with its own deliverable-kW, delivered-kWh, response and reserve-recovery evidence. Never compare DJI's controlled 25-second swap with another vendor's airborne launch time or with a fleet-level reliability percentage.

Du & Zhu's 16 November 2026 study is especially important as a current learned-policy comparator. It formulates heavy-truck swapping-station scheduling as a KL-ambiguity-set robust MDP; an offline VAE estimates the nominal transition distribution, and DR-SAC emits integer SOC-level charging-count decisions. The paper reports 3.7% relative daily-profit variation under its joint demand/price perturbation and 78.9% peak shaving for its selected parameterization. In its high-demand scenario, however, DR-SAC still records 5.3 constraint-violating periods/day; DDPG achieves 86.7% peak shaving but 21.7 violating periods/day. The authors explicitly identify the need for a safety layer under high demand. The study uses one mining-area station's non-public operating data, five training seeds and 31 test days, and lists VAE tail-risk under sparse states, fixed 15-minute transitions/constant-power charging and absent SOH degradation as limitations. These are not apples-to-apples UAV results, but they give the Luminous tournament a very concrete challenge: keep robust-policy economics while placing an independent, deterministic reserve gate between a learned proposal and any admissible dispatch. [Du & Zhu, 2026](https://doi.org/10.3390/app162211346)

For that reason the benchmark must report the count and severity of reserve/mission violations beside peak shaving and profit—not infer safety from the average. The current v0.1 verifier does not solve the robust MDP or provide an out-of-distribution guarantee; it is a proposal shield against the finite, explicit scenarios supplied to it. To claim broad robustness, v0.2/v0.3 must add unseen/correlated shocks and reserve recovery, while the source evidence and scenario-generation assumptions remain independently reviewable.

The benchmark should reproduce each paper's own system first, freeze its assumptions, and compare published and reproduced results separately. Only then should it transplant the algorithms into a new, matched UAV/carrier dataset. Never place different grid topologies, battery classes, reserve products or objectives on one percentage leaderboard.

## 5. How to compete—and where to aim higher

The field's strongest methods often optimize the same coupled problem that matters here: demand, charge scheduling, renewable uncertainty, price and grid limits. The differentiator should not be a novel optimizer label. It should be an independently testable **assurance boundary** around what the optimizer may offer.

### Baseline tournament

1. Fixed-cap, mobility-only charging.
2. PV + stationary ESS self-consumption, no market dispatch.
3. Deterministic TOU/day-ahead MILP.
4. Rolling-horizon MPC.
5. Two-stage robust optimization / column-and-constraint generation.
6. Wasserstein or other distributionally robust optimization.
7. DR-SAC / offline RL policy proposal.
8. Cyber-physical forecast-and-optimization baseline with delay and data attacks.
9. Each optimizer with and without the exact same reserve-envelope shield.

Use identical frozen inputs, reserve scenarios, forecasts, initial conditions, time limits, solver settings and compute budgets. Include out-of-distribution demand spikes, renewable shortfall, charger/ESS outage, site import derating, telemetry delays, replayed values, forged export limits and network partitions.

### Success metrics

| Family | Primary measure | Failure boundary |
| --- | --- | --- |
| Mobility | committed missions served; pack stockout probability; p95/p99 queue and delay | no mission or inventory reserve encroachment |
| Flexibility | deliverable kW, deliverable kWh, response error and duration | no per-slot or cumulative energy violation |
| Recovery | time to restore reserve after a grid event; ability to survive a second event | no declaring service restored while reserve remains depleted |
| Grid | metered PCC import/export, peak demand and losses | no export without exact connection and permission evidence |
| Cyber | detection/abstention on stale, replayed, false and inconsistent inputs | zero false acceptance of hard-limit violations |
| Economics | lifecycle cost per successful on-time mission and per verified grid-service kWh | no unverified market revenue in the base case |
| Computation | optimality gap, runtime p50/p95/p99, memory and timeout rate | timeout/unknown result cannot silently become feasible |

### Target ladder (not achieved results)

- **Small exact family:** 100% agreement with an independently implemented enumerator.
- **Hard safety negatives:** zero false acceptance over every frozen negative fixture and adversarial evidence mutation.
- **Replay:** deterministic canonical output under input permutations where the schema defines a unique ordering.
- **Bounded v0.1 runtime:** target p99 below 10 ms for at most four resources, four scenarios and eight slots on a declared runner. Record hardware and workload; do not report this until measured.
- **Larger optimization family:** mean regret at or below 1% and p95 at or below 5% against exact/MILP references where available. Report timeout cases separately.
- **Resilience:** preserve all declared mobility obligations under every admitted scenario; publish the minimum reserve margin, not just average utilization.
- **Economic superiority:** lower full lifecycle cost per successful on-time mission while reporting the opportunity cost of any grid flexibility delivered.

The first scoreboard must prioritize zero hard violations over economic return. A policy that earns more by consuming a safety reserve is not a more efficient solution; it is infeasible.

## 6. Further paradigm shifts

### A. Proof-carrying flexibility

The eventual offer to a grid aggregator should be a bounded, versioned certificate containing the exact resource set, configuration digests, mission-reserve snapshot, reserve-scenario set, PCC envelope, time window, candidate trajectory, expiry and input digests. The downstream scheduler should consume that certificate once and reject replay or overlapping commitments. It remains a qualification receipt—not an actuation command.

### B. Reserve is non-fungible across time and assets

Do not pool every battery into a single kWh number. A resource may be at a different site, have a different interface, or be unavailable in the relevant time window. Reserve belongs to a resource/configuration and a time interval. Cross-resource aggregation is valid only after a site-connection and delivery-path qualification.

### C. Measure reserve recovery, not just service recovery

A depot can return to 100% of its nominal mission service while its storage/inventory buffer remains depleted. It may then fail during a second disturbance. v0.2 should model the recharge/recovery horizon and compare isolated single shocks with correlated/repeated shocks. Publish both service restoration and reserve restoration curves.

### D. Make uncertainty adversarial and common-cause aware

Do not assume demand error, renewable shortfall, communication delay and charger outage are independent. Generate correlated scenarios and common-cause cuts: site meter failure, common firmware, shared network, shared power bus, weather system, or one unavailable technician/vehicle. The reserve shield should use a declared scenario family and make unknowns visible.

### E. Let learning propose; let independent evidence constrain

DR-SAC, LSTM, forecasting and RL can propose useful policies. They must not write the reserve ledger, mint qualifications or command contactors. Compare them behind an unchanged deterministic feasibility gate, and measure the performance loss caused by the gate separately from the number and severity of unsafe proposals rejected.

### F. Co-design mobility and site infrastructure without blending authorities

Capacity planning can jointly examine aircraft/vehicle inventory, mobile service units, PV, stationary storage, charging channels, thermal conditioning, grid connection and backup. But site PCC metrology remains #5832, battery custody remains Mycelix, electrical protection remains local, and flight permissions remain #6083. The optimizer should never become the authority merely because it sees more variables.

## 7. Standards and South African deployment boundary

- **IEEE 1547-2018** remains an active DER interconnection/interoperability reference covering abnormal conditions, power quality, islanding and testing. The IEEE SA description says it is written for a 60 Hz source; it is not a direct substitute for South African 50 Hz grid-code, NRS and licensed-distributor requirements. [IEEE SA standard page](https://standards.ieee.org/ieee/1547/5915/).
- **ISO 15118-20:2022** defines road-EV/EVSE communication for unidirectional and bidirectional power transfer. It is a communications comparator for road vehicles, not proof that an aircraft propulsion pack, BMS, inverter or charger is safe or qualified for V2G. [ISO standard page](https://www.iso.org/standard/77845.html).
- South African deployment must keep embedded-generation registration, technical compliance, fee-waiver eligibility, export permission, third-party wheeling and future SAWEM participation separate. The current #6707 dossier records National Treasury's April 2026 internal SAWEM start with external trading planned for early 2027, Stats SA's April 2027 planned launch, and the distinction between the 100 kW registration category and an Eskom 50 kVA registration-fee waiver. These are not permission to export an aircraft battery.
- The near-term commercial baseline should be approved behind-the-meter self-consumption, measured peak management and site resilience. Treat physical wheeling, virtual wheeling, wholesale settlement and V2G/export as separate conditional cases requiring the exact site, equipment, connection, metering, technical approval and market evidence.

The business case must not include SAWEM or export revenue merely because an optimizer has computed a positive flexibility envelope.

## 8. Next phases

### v0.2 — recovery-qualified trajectory feasibility

- add a post-dispatch recovery window with charge-power headroom and qualified recovery-path evidence;
- require the mobility buffer to be restored before the next mission deadline;
- model interval-dependent efficiency and thermal/operating derating only where source-qualified measurements exist;
- bind each candidate dispatch to the #6700 occupancy/reconciliation snapshot and consume-once receipt;
- add a two-disturbance test: dispatch event, recovery period, second outage before full restoration.

### v0.3 — exact small-instance dispatch oracle

- enumerate per-resource dispatch allocations, not just site aggregates;
- compare greedy, MPC, MILP/robust and DRO schedules against the same exact cases;
- include demand response by charging deferral as a separate service from storage discharge;
- add common-cause resource groups and deterministic conflict detection.

### v0.4 — grid-network and field evidence

- import qualified feeder topology and load-flow constraints;
- compare site-only feasibility to AC power-flow and operator network studies;
- use independently metered PCC trajectories and exact charge/discharge efficiency;
- measure response, sustain, accuracy, recovery, losses and degradation with declared denominators;
- preserve independent protection and commissioning requirements.

## 9. Claim boundary

A green v0.1 fixture means only that a synthetic candidate profile satisfies the supplied finite reserve, power and PCC arithmetic. It does not demonstrate field resilience, real storage availability, a qualified bidirectional aircraft battery, V2G, grid export, interconnection, power quality, islanding, black start, market eligibility, aircraft safety or production readiness.
