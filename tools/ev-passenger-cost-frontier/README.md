# EV Passenger Cost Frontier v0.1

A small Rust tool for turning a retail-price target into an explicit allowable vehicle cost envelope. It complements, and does not replace, the Symtropy EV mission/energy/charging screens.

## Run

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
cargo run --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- \
  tools/ev-passenger-cost-frontier/fixtures/ev_passenger_cost_v0_1.json
```

## Cost boundary

All monetary values are in the catalog currency. Every BOM line carries a quantity unit and unit-cost basis (for example 42 kWh of cells at ZAR/kWh plus a vehicle-set cost for enclosure and BMS). Each cost range must be **landed at the assembly facility** and include the applicable supplier price, currency conversion, inbound freight, duties and non-recoverable charges. Factory conversion and warranty/service reserve are per good vehicle. Fixed program investment is amortized over the declared number of vehicles. VAT and the dealer/distributor allowance are applied only in the retail waterfall.

- Retail ceiling = observed like-for-like baseline retail price × (1 − target undercut).
- Ex-VAT retail = retail ceiling / (1 + VAT), if the baseline includes VAT.
- OEM net revenue = ex-VAT retail × (1 − dealer/distributor margin).
- Maximum affordable COGS = OEM net revenue × (1 − target OEM gross margin).
- Good-vehicle BOM = sum(quantity × unit-cost range) / manufacturing yield.
- COGS = good-vehicle BOM + per-good-unit conversion + warranty/service reserve + fixed program investment / amortization units.
- Implied retail price inverts the same margin/tax waterfall for each COGS bound.

The model uses cost *ranges*. A low-case result cannot conceal a high-case shortfall.

## Public component research

- Research dossier: [EV Component Public Benchmarks and Improvement Opportunities](../../docs/research/EV_COMPONENT_PUBLIC_BENCHMARKS_AND_IMPROVEMENT_OPPORTUNITIES_V0_1.md)
- Machine-readable source ledger: `fixtures/ev_component_public_benchmarks_v0_1.json`

This evidence is gathered from public market surveys, online retail listings, manufacturer specifications and official tariff/policy sources. It does not contact vendors or imply firm quotes. The ledger labels the exact evidence class and claim limitations for each source.

## Evidence and claim limit

Evidence classes such as `synthetic`, `estimate`, and `literature` are scenario-building inputs, not production quotes. Only manufacturer quotes, measured costs or audited cost records count as cost-release-grade in v0.1. A candidate is never production-feasible unless its price envelope, engineering gate, safety gate and vehicle-envelope evidence all qualify. Passing this CLI is arithmetic qualification only; it does not demonstrate manufacturability, safety, homologation or achievable MSRP.

The committed fixture deliberately uses synthetic costs and unqualified gates, so every candidate must remain `needs_evidence`. It illustrates the method, not an assertion that any design can be produced for the target price.


## EV-CARRIER-MODULE-MFG-001 — process capacity overlay

The `ev_carrier_module_factory_screen` binary consumes the existing `ev_factory_frontier` report and adds incremental manufacturing routes for carrier modules. It compares a synthetic detachable UAV pod, synthetic AMR handoff module and an intentionally unknown integrated drone-bay route.

Run after `ev_factory_frontier`:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin ev_carrier_module_factory_screen -- \
  ev-factory-design-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_carrier_module_processes_v0_1.json
```

The overlay inherits productive minutes/cell-year, shifts, target good-carrier throughput and base factory release gaps from the factory report. It computes module attempts from modules-per-carrier and module yield, sizes required cells against declared available parallel cells, identifies the process bottleneck, and reports additional operators, active shift positions, footprint, equipment capex and module energy per good unit. Unknown process data remain indeterminate; a known cell-capacity shortfall blocks the route.

This is an incremental capacity/resource overlay, not a second factory costing engine. Base building, general overhead, site labor/energy rates and vehicle assembly costs remain with `ev_factory_frontier`. Module production release remains false in v0.1, including synthetic-screen passes.

Research dossier: [EV Carrier Module Manufacturing Screen v0.1](../../docs/research/EV_CARRIER_MODULE_MANUFACTURING_SCREEN_V0_1.md).


## EV-CARRIER-LIFECYCLE-001 — module lifecycle and serviceability screen

The `ev_carrier_lifecycle_screen` binary consumes `ev_carrier_module_factory_screen` output and adds module build cost, scheduled inspection burden, fault intervention repair-versus-replacement comparison, parts wait, technician hours and per-module maintenance time fraction.

Run after the module factory overlay:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin ev_carrier_lifecycle_screen -- \
  ev-carrier-module-factory-report.json \
  tools/ev-passenger-cost-frontier/fixtures/ev_carrier_lifecycle_v0_1.json
```

The unscheduled event rate is calculated only from an explicit event count divided by an explicit module-year exposure denominator. The tool does not infer failures from synthetic task examples. A stale/unknown post-maintenance test cannot qualify a service path; the evaluator never grants return-to-service authority or production release. The single-module maintenance-time fraction is not fleet availability.

Dossier: [EV Carrier Lifecycle and Serviceability Screen v0.1](../../docs/research/EV_CARRIER_LIFECYCLE_AND_SERVICEABILITY_V0_1.md). Compose generic task semantics from [AERO-SERVICE-LOOP-001 #4056](https://github.com/Luminous-Dynamics/symtropy/issues/4056) and lifecycle comparisons from [EV-BENCH-001 #2164](https://github.com/Luminous-Dynamics/symtropy/issues/2164).


## BATTERY-SERVICE-NETWORK-001 — exact one-epoch pack allocation v0.1

The `battery_service_network_screen` binary is the first executable slice for [BATTERY-SERVICE-NETWORK-001 #6674](https://github.com/Luminous-Dynamics/symtropy/issues/6674).

Research dossier: [Battery Service Network Benchmark v0.1](../../docs/research/BATTERY_SERVICE_NETWORK_BENCHMARK_V0_1.md).

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin battery_service_network_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/battery_service_network_v0_1.json
```

The v0.1 exact evaluator assigns at most one qualified pack to each mission in one simultaneous dispatch epoch. It enumerates assignments for at most six packs and six missions, maximizing service count, then minimizing energy surplus, then applying a deterministic identifier tie-break.

For each candidate pair it independently checks:
- exact pack/mission configuration identifier and configuration digest;
- pack operational state carried inside its time-bound evidence envelope;
- pack identity, custody, and post-test evidence bound to the exact pack ID and configuration digest;
- source/producer ID, method ID, evidence reference, observation timestamp and explicit validity interval for every evidence envelope;
- evidence observed no later than the declared dispatch epoch, with the epoch inside the evidence validity interval (future observations are blocked; expired or incomplete validity windows are indeterminate);
- available state and fresh passed post-test evidence;
- conservative lower bound for usable energy against mission energy **including reserve**;
- conservative lower bound for peak power against the mission peak-power requirement;
- conservative upper bound for pack temperature against both mission and qualified pack limits.

Point estimates are not accepted as capability bounds: the energy/power inputs are explicitly conservative lower bounds, while temperature is an explicit conservative upper bound. Unknown/stale/incomplete evidence is indeterminate; known identity/configuration mismatch, future observation, non-available/quarantined state, failed post-test or exceeded energy/power/thermal envelope is blocked. Capacity cannot compensate for insufficient peak power. Each pack can be assigned only once in the common epoch. Evidence references and producer/method identifiers are declarative strings in this synthetic benchmark; cryptographic authenticity and authoritative identity/custody resolution remain external owner responsibilities.

The machine-readable input contract is `schemas/battery_service_network_v0_1.schema.json`; the frozen known-answer is `fixtures/battery_service_network_v0_1.json`. The focused workflow runs Rust formatting, tests, CLI evaluation and JSON assertions, and retains a report/hash artifact.

## CHARGER-QUEUE-ORACLE-001 — exact fixed-duration charging queue

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin battery_charge_queue_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/battery_charge_queue_v0_1.json
```

This is a bounded exhaustive scheduler for up to four jobs, three charger channels and eight integer time slots. It models each charge as a fixed-duration, fixed-input-power block; requires the pack to be evidence-bound and in Cooling/charge-ready state at the planned start; requires exact pack-configuration digest compatibility; enforces one job per charger channel at a time; and enforces per-slot charging power no greater than site import cap less non-charging site load. Charger and site evidence must cover the horizon; grid evidence must cover each full slot.

The objective is lexicographic: maximize completed jobs, minimize total waiting slots, minimize makespan, then deterministic identifiers.

The charger maximum power and supported pack-configuration digests are nested inside a separate time-bound capability evidence record; charger availability is also evidenced across the horizon. Each grid slot carries its import cap and non-charging site load inside an evidence envelope bound to the exact site/configuration and covering the whole slot. Pack cooling-state evidence is checked at the candidate start time. These values are still declarative inputs in this prototype; cryptographic payload authentication and independent metering are not implemented.

Charger capability (maximum power and exact supported pack configuration digests) and each grid slot's import cap/base load are nested inside their own evidence-bearing records, bound to exact charger/site identity and configuration. Charger/site evidence covers the whole scheduling horizon; each grid evidence record covers its full slot. Pack cooling-state evidence must remain valid at the proposed charge start. It reports declared grid input in watt-seconds (= requested charger input power × duration seconds). This is not pack-terminal energy: charging efficiency, charge curves, battery degradation and thermal evolution are not modeled.

The synthetic fixture has four jobs, two charger channels and six 15-minute slots. Its exact known answer schedules:
- job-a / pack-a on charger-1, slots [0,2), 3,000 W;
- job-d / pack-d on charger-2, slots [0,3), 2,500 W;
- job-b / pack-b on charger-1, slots [2,4), 3,500 W;
- job-c is unserved because its release/deadline window is shorter than its three-slot fixed charge block.

Site import cap is 8,000 W per slot and declared non-charging load is 1,000 W, leaving at most 7,000 W for charging. The known-answer report has 54 exact-search nodes and 18,450,000 declared watt-seconds of grid input across the three scheduled jobs.

Schema: `schemas/battery_charge_queue_v0_1.schema.json`; fixture: `fixtures/battery_charge_queue_v0_1.json`; CLI: `src/bin/battery_charge_queue_screen.rs`.

## PACK-STATE-TRANSACTION-001 — evidence-bound pack event journal

This v0.1 event-sourced reducer is tracked in [#6700](https://github.com/Luminous-Dynamics/symtropy/issues/6700).

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin battery_pack_journal_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/battery_pack_state_journal_v0_1.json
```

The reducer validates a frozen initial pack state plus a monotonic event sequence. A confirmed event applies only an allowed state transition. A timeout or unknown outcome preserves the last confirmed state and sets `reconciliation_required`; no subsequent normal event is accepted until a reconciliation event carrying measured or independent-test evidence confirms the observed state. At the replay cut-off, the latest confirmed state evidence must still be within its validity interval; otherwise the report also requires reconciliation. Event IDs and idempotency keys are unique, sequences contiguous, timestamps monotonic and no later than the replay cut-off, and evidence must bind exact serial/configuration and its event time.

The synthetic fixture demonstrates: available → reserved (confirmed), an install operation times out (state remains reserved and locked), then independent-test reconciliation observes installed and clears the lock. This is not proof that an actual swap occurred. It models how to preserve the uncertainty and block a speculative retry.

Schema: `schemas/battery_pack_state_journal_v0_1.schema.json`; fixture: `fixtures/battery_pack_state_journal_v0_1.json`; CLI: `src/bin/battery_pack_journal_screen.rs`.

## Exclusive resource occupancy screen (#6700)

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin battery_resource_occupancy_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/battery_resource_occupancy_v0_1.json
```

The validator checks time-indexed **exclusive** occupancy of serialized packs, aircraft, carrier slots, swap bays, chargers/channels, racks, technicians and mobile service units. Intervals are half-open `[start,end)`: adjacent operations are allowed; true overlap on the same resource kind + ID is rejected. Every interval needs unique occupancy/idempotency IDs, owner/operation identifiers, exact resource/configuration binding, and evidence observed no later than the schedule decision time whose validity covers the whole reservation interval.

The synthetic fixture contains four reservations across three resources. A pack and its aircraft may be occupied simultaneously because they are distinct resource identities; two swap-bay intervals are contiguous and share a boundary without overlap. The report sorts occupancy deterministically by resource kind, ID, start/end and occupancy ID.

Schema: `schemas/battery_resource_occupancy_v0_1.schema.json`; fixture: `fixtures/battery_resource_occupancy_v0_1.json`; CLI: `src/bin/battery_resource_occupancy_screen.rs`.

**Deliberate scope limit:** this is a static exact assignment and evidence gate, not a multi-period charging/rack queue optimizer, mobile battery-service routing model, degradation model, thermal battery model or aircraft dispatch authorizer. Those are the next layers in #6674. Synthetic fixture passes validate accounting/algorithm semantics only and do not establish real battery capability, flight safety or operational release.


## MOBILITY-FLEX-ENVELOPE-001 — time-coupled reserve-first flexibility verifier

The `mobility_flexibility_envelope_screen` binary is the first executable child of [MOBILITY-ENERGY-RESILIENCE-001 #6707](https://github.com/Luminous-Dynamics/symtropy/issues/6707), tracked separately in [#6802](https://github.com/Luminous-Dynamics/symtropy/issues/6802).

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin mobility_flexibility_envelope_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/mobility_flexibility_envelope_v0_1.json
```

The verifier consumes time-bound site, pack-allocation, charger-queue, PCC-meter, storage-capability and site-connection evidence. The proposal includes both a site/PCC power profile and an explicit per-resource, per-slot allocation; the two must reconcile exactly. For every reserve scenario, it subtracts resource-specific mission, inventory-buffer, safety/outage and uncertainty energy/power reserves. It tests each asset's own cumulative energy and slot power budget before aggregating the resulting trajectory at the PCC. It tests the proposal against:
- each resource's cumulative residual energy budget across the whole horizon;
- each resource's residual discharge-power ceiling in every slot;
- the aggregate horizon budget and aggregate per-slot ceiling as cross-checks;
- the site load plus the charging schedule at the PCC;
- import limits, export limits and explicit export-permission evidence;
- unique reserve receipt IDs within each mutually exclusive scenario.

This closes two gaps: positive aggregate kWh cannot conceal a per-slot kW violation, and one asset's unused energy or power cannot be borrowed to mask another asset's shortfall. Per-resource energy is calculated from each resource/slot interval with upward Wh rounding; per-slot resource dispatch must stay inside that asset's reserve-adjusted power ceiling. The site profile is accepted only when it equals the sum of those resource allocations. Each of the four reserve buckets carries an evidence envelope bound to the exact resource/configuration, horizon and corresponding receipt ID. The per-slot non-charging load is bound to the site-meter receipt; the per-slot charging load is bound to the exact charging-queue report reference. The same reserve receipt cannot be counted twice inside a scenario, and unbound per-slot loads are rejected. A resource must be bound both to its own exact configuration and to the site's exact configuration. Aircraft propulsion packs are rejected as grid resources; bidirectional storage requires independent-test interface evidence. A positive export permission must itself carry non-synthetic, non-unknown evidence; the report still never grants export authority. The v0.1 PCC model has no separate PV-generation, feeder-topology or AC power-flow model; any supplied base-site load must be the declared net non-charging load at the PCC.

The frozen two-asset known answer allocates the site trajectory [2,200, 2,500, 2,000, 800] W as ess-01 [2,000, 1,700, 1,300, 300] W and ess-02 [200, 800, 700, 500] W. In the high-demand/low-renewable scenario, ess-01 has 1,450 Wh residual and uses 1,325 Wh (125 Wh margin); ess-02 has 550 Wh residual and uses all 550 Wh (0 Wh margin). This demonstrates why per-asset—not just pooled—energy and power checks are required. The module includes an independent reference calculation that exhaustively compares the evaluator against 625 four-slot candidate site profiles using a frozen two-asset allocation rule (158 expected feasible, 467 expected rejected). This test passed in exact-head Actions run [#45026367607](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45026367607): the package's Cargo test binaries reported 130 passed, 0 failed; workspace formatting and schema/fixture parsing also passed. The workflow retained the deterministic JSON report as an artifact. Clippy is not yet part of this workflow. The screen is a feasibility verifier, not a schedule optimizer. Its report always sets `dispatch_authority_granted=false` and `export_authority_granted=false`. The frozen fixture uses synthetic data and therefore demonstrates arithmetic and rejection semantics only; it is not a real site capability claim, a metered energy result or permission to discharge/export.

Schema: `schemas/mobility_flexibility_envelope_v0_1.schema.json`; fixture: `fixtures/mobility_flexibility_envelope_v0_1.json`; evaluator: `src/mobility_flexibility_envelope.rs`; CLI: `src/bin/mobility_flexibility_envelope_screen.rs`.

Research dossier: [Mobility-First Grid Flexibility Envelope Benchmark v0.1](../../docs/research/MOBILITY_FLEXIBILITY_ENVELOPE_BENCHMARK_V0_1.md). The v0.1 model does not yet certify post-dispatch recharge/recovery viability, full AC power flow, thermal evolution, degradation, or market deliverability; these are explicit later phases, not implied by a passing synthetic profile.


## MOBILITY-RESERVE-RECOVERY-001 — chronological reserve restoration (v0.2 prototype)

The recovery child of [#6707](https://github.com/Luminous-Dynamics/symtropy/issues/6707) is tracked in [#6808](https://github.com/Luminous-Dynamics/symtropy/issues/6808) and builds on the offer-window flexibility check in [#6802 / PR #6803](https://github.com/Luminous-Dynamics/symtropy/pull/6803).

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin mobility_recovery_envelope_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/mobility_recovery_envelope_v0_2.json
```

The evaluator replays an eight-slot, two-resource timeline. For each scenario and asset it debits mission, disturbance and grid-delivery energy before crediting conservative lower-bound charging energy for that interval. Charge Wh is rounded down; discharge Wh is rounded up. It checks the starting reserve floor, every subsequent mission/inventory/safety/uncertainty floor, charger compatibility and exclusive channel occupancy, scenario-specific channel availability, resource charge/discharge power, capacity, PCC import headroom and the exact recovery target deadline.

The frozen second-shock case yields:

| Resource | Energy at slot 5 | Recovery target | Margin |
| --- | ---: | ---: | ---: |
| ess-01 | 1,151 Wh | 1,100 Wh | +51 Wh |
| ess-02 | 960 Wh | 950 Wh | +10 Wh |

The site's modeled import profile is [1,900, 1,900, 4,300, 4,300, 2,500, 4,000, 2,500, 2,500] W against a 6,500 W limit. The two-scenario synthetic fixture includes a second event before reserve restoration is complete. It demonstrates arithmetic and rejection behavior only; it does not demonstrate observed charging, measured state of energy or an operational recovery guarantee.

This is a source-level prototype with 11 focused Rust test definitions at this branch revision. The existing workflow has been wired to parse the strict schema/fixture, run the CLI, assert the known-answer states and PCC profile, and retain a JSON report. **Rust formatting, compilation, tests and CI are not yet verified for this recovery branch.** The report always has dispatch and export authority set to false.

Schema: `schemas/mobility_recovery_envelope_v0_2.schema.json`; fixture: `fixtures/mobility_recovery_envelope_v0_2.json`; evaluator: `src/mobility_recovery_envelope.rs`; CLI: `src/bin/mobility_recovery_envelope_screen.rs`.

Research dossier: [Mobility Reserve Recovery and Repeated-Shock Benchmark v0.1](../../docs/research/MOBILITY_RESERVE_RECOVERY_BENCHMARK_V0_1.md). The current model does not certify cell electrochemistry, thermal/SOH derating, AC feeder flow, metered charging completion or the cryptographic authenticity of its declarative evidence envelopes. It is a candidate-trajectory evaluator, not a charge command or a return-to-service certificate.


## MOBILITY-COMMITMENT-LEASE-001 — cross-domain capability reservation

The commitment-lease child of #6707 is tracked in [#6827](https://github.com/Luminous-Dynamics/symtropy/issues/6827). It follows the verified flexibility and recovery gates in [PR #6803](https://github.com/Luminous-Dynamics/symtropy/pull/6803) and [PR #6809](https://github.com/Luminous-Dynamics/symtropy/pull/6809).

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin mobility_commitment_lease_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/mobility_commitment_lease_v0_1.json
```

The evaluator consumes:
- source-bound flexibility and recovery report digests;
- per-resource safe energy budgets and per-slot safe power ceilings, already net of mobility and recovery reserves;
- protected mission, recovery, maintenance and safety-hold intervals;
- existing external commitments with exact resource/configuration, state, idempotency key, expiry and per-slot allocations;
- one candidate firm lease.

`firm_offered`, `accepted`, `executing` and `unresolved` states reserve capability. A timeout does not release a resource. `rejected`, `expired`, `cancelled` and `settled` records release capacity only when their state evidence is measured or independently tested. The evaluator does not infer that an offer has expired merely because its clock elapsed; the state transition/release must be supplied with evidence. Back-to-back leases use half-open windows `[start_slot, end_slot)`.

Each resource is checked independently against its own cumulative energy budget and per-slot power ceiling. The evaluator rejects reused commitment IDs/idempotency keys, configuration mismatches, protected-window overlap, stale report evidence, malformed intervals and resource-level overcommit.

The frozen fixture contains two active commitments (including one unresolved timeout), one independently evidenced settled record, one mission-protected interval and one new back-to-back candidate. It yields:
- ESS-01: 1,100 Wh committed out of 1,500 Wh; slot powers [500, 500, 2,500, 1,500, 0, 0] W; remaining energy 400 Wh.
- ESS-02: 300 Wh committed out of 900 Wh; slot powers [0, 500, 500, 0, 0, 0] W; remaining energy 600 Wh.

This is a proposal-only admission calculation. It does not write to a ledger, reserve a physical resource, make a market offer, authorize dispatch or establish settlement. The upstream report references and evidence envelopes are declarative; cryptographic verification and consume-once persistence remain later integration work.

The module currently includes 16 focused Rust test definitions, including a 12-profile independent capacity reference, unresolved-state reservation, terminal evidence, per-resource energy/power limits, protected mission windows, idempotency reuse and deterministic input-order replay. Runner verification is pending for the commitment branch.


## MOBILITY-COMMITMENT-JOURNAL-001 — CAS append-only promise journal

The persistence/concurrency child of [#6827](https://github.com/Luminous-Dynamics/symtropy/issues/6827) is tracked in [#6832](https://github.com/Luminous-Dynamics/symtropy/issues/6832).

Run the frozen four-event scenario:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin mobility_commitment_journal_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/mobility_commitment_journal_v0_1.json
```

The reducer stores each transition as an append-only event with a monotonically increasing sequence, expected-version precondition, previous-event digest, SHA-256 event digest, exact commitment ID, idempotency key, immutable contract payload digest and event evidence. Replay checks sequence continuity, event IDs, idempotency uniqueness, previous-event links, event digests and the commitment state machine.

The test-only `InMemoryCommitmentJournal` adapter serializes compare-and-append through a mutex. Two concurrent writers at the same ledger version must produce exactly one winner; the other receives a version conflict. This is a local concurrency test, not a distributed consensus or durable-storage claim.

Identical retry with the same idempotency key, command kind, commitment and contract payload returns the original receipt without increasing the version—even when the original response may have been lost. Reusing the same key with a different payload or state command is a hard conflict.

An unknown physical/market outcome becomes `unresolved`; it remains active until an explicit `reconciled` event. Settled/rejected/expired/cancelled release requires measured or independent-test evidence for every exact resource/configuration in the immutable contract. There is no timer-based capacity release.

The frozen fixture replays:
`offer_created → accepted → executing → unresolved`
and finishes at ledger version 4. It deliberately leaves `lease-001` unresolved and reserved. The CLI report sets ledger mutation, market, dispatch and physical mutation authority to false.

The module currently contains 14 focused Rust test definitions covering concurrency, idempotency, payload conflict, unresolved retention, terminal evidence, invalid transition, stale writer, digest mutation, previous-link mutation and event reordering. SHA-256 here provides an integrity-linked event chain; it does not authenticate producers or prove durable storage. Exact-head Actions run [#45042240520](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45042240520) passed schema/fixture parsing, workspace formatting, Cargo tests, deterministic report assertions and artifact upload (171 tests passed, 0 failed across test binaries). Clippy is not included and remains unverified.

Source: `src/mobility_commitment_journal.rs`; CLI: `src/bin/mobility_commitment_journal_screen.rs`; strict schema: `schemas/mobility_commitment_journal_v0_1.schema.json`; fixture: `fixtures/mobility_commitment_journal_v0_1.json`.


## SUSTAINABLE-MOBILITY-SERVICE-CAPACITY-001 — cross-architecture operating benchmark

The benchmark dossier is `docs/research/SUSTAINABLE_MOBILITY_SERVICE_CAPACITY_BENCHMARK_V0_1.md`. It extends the existing industry baseline [#6226](https://github.com/Luminous-Dynamics/symtropy/issues/6226) and matched-instance tournament [#6247](https://github.com/Luminous-Dynamics/symtropy/issues/6247), rather than creating duplicate ownership.

The central rule is to report six independent service denominators: mechanical swap success, landing-to-launch turnaround, charge restoration, qualified resource availability, successful on-time missions and independently metered grid service.

The same assurance sequence is now applied to all optimizer proposals:
- [#6803](https://github.com/Luminous-Dynamics/symtropy/pull/6803): per-resource offer-window flexibility;
- [#6809](https://github.com/Luminous-Dynamics/symtropy/pull/6809): chronological recovery and repeated-shock viability;
- [#6829](https://github.com/Luminous-Dynamics/symtropy/pull/6829): unique cross-domain capability leases;
- [#6837](https://github.com/Luminous-Dynamics/symtropy/pull/6837): compare-and-swap promise journal and idempotent replay.

DJI Dock 4's published <25 s swap and <45 s landing-to-takeoff turnaround are distinct from its 27 min 15–95% charging-time specification at 25°C. A fast mechanism is not evidence of sustainable fleet throughput. See the dossier for the matched workload, metric boundaries, SOTA tournament, hard gates and target ladder.


## SUSTAINABLE-MOBILITY-SERVICE-CAPACITY-001 — bounded cyclic throughput evaluator

The executable child of [#6909](https://github.com/Luminous-Dynamics/symtropy/issues/6909) composes mission duration, operational turnaround, ready spare inventory, fixed-duration charger channels and site import headroom into a deterministic event timeline.

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin sustainable_mobility_service_capacity_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_capacity_v0_1.json
```

The frozen eight-hour fixture has four profiles:
- direct one-pack recharge;
- a Dock 4 specification-shaped profile (one aircraft, three stored spares, two channels, 45-second turnaround, 27-minute restoration); five-minute mission duration and 500 W per-channel input power are synthetic stress assumptions;
- the same profile with site headroom reduced to one effective charging channel;
- a two-aircraft shared compatible pool, with three effective channels after site-load headroom.

At each mission completion the depleted pack enters an FCFS charge queue. The aircraft consumes a ready spare or waits. A completed charging job becomes a ready spare or is assigned to the oldest waiting aircraft. The site grid cap limits concurrent charging channels. Event ties are deterministic: charge completions are processed before mission completions, then by channel/aircraft ID.

The output reports completed modeled sorties, cycles/hour, mission-hours, effective channels, queue peaks, charging starts/completions, aircraft wait, end-of-horizon inventory/queue state and a charger-input energy proxy. A 1-pack/no-recharge closed-form test independently checks the mission-completion schedule; a pack reuse bound checks that completed sorties cannot exceed initial packs plus completed recharges.

Important: all modeled sorties are assumed to complete. v0.1 omits demand arrivals/deadlines, mission failures, weather, aviation permissions, maintenance, physical charge curves, battery aging, crew, mobile replenishment travel and cost. The 500 W charging-channel input proxy and site fixed load are synthetic; the Dock 4's published 2 kW is a maximum dock AC input, not a per-channel charging rating. This is capacity semantics only, not field service capacity or a DJI performance reproduction.

Source: `src/sustainable_mobility_service_capacity.rs`; CLI: `src/bin/sustainable_mobility_service_capacity_screen.rs`; schema: `schemas/sustainable_mobility_service_capacity_v0_1.schema.json`; fixture: `fixtures/sustainable_mobility_service_capacity_v0_1.json`.

Exact-head Actions run [#45068027684](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45068027684) at `28b353addd2c4da8b9896516a145f82a78af7595` passed strict schema/fixture parsing, workspace formatting, Rust tests, deterministic report assertions and artifact upload. Test binaries reported 156 + 16 + 8 + 9 = **189 passed, 0 failed**; all 18 service-capacity tests passed, including an independent second-by-second tick oracle across 20 bounded cases, pack-conservation controls and the finite-horizon stock-drawdown bound. All profiles use the same synthetic five-minute mission and 45-second turnaround. Eight-hour outputs: direct single-pack 15 sorties (1.875 per aircraft-hour); Dock-shaped two-channel 38 (4.75); grid-limited one-channel 21 (2.625); two-aircraft shared pool 59 (7.375 fleet cycles/hour, 3.6875 per aircraft-hour). Steady-state ceilings: 2.222, 4.444, 2.222 and 6.667 cycles/hour. These are model outputs, not field/customer service claims; every launched sortie is assumed to complete. Clippy remains unverified.


## SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001 — request deadlines and fallback service

The request-quality extension of [#6909](https://github.com/Luminous-Dynamics/symtropy/issues/6909) is tracked in [#6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921).

The legacy v0.1 fixture remains available for reproduction:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin sustainable_mobility_service_quality_screen -- --legacy \
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_quality_v0_1.json
```

The preferred linked mode takes a request/payload-evidence fixture and the exact capacity architecture input separately:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin sustainable_mobility_service_quality_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_quality_linked_v0_1.json \
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_capacity_v0_1.json
```

This is a deterministic single-site request-to-service-opportunity evaluator. It consumes a declared #6909 capacity envelope and prequalified time-bounded aerial/fallback service opportunities. It does not run routes or charging simulation, and it does not select strategic operating modes.

Both FCFS and EDF are evaluated over the same arrivals, deadlines, service opportunities, permission/weather windows and blocked intervals. Requests arriving after their acceptance cutoff are rejected. Starts after a request deadline are not admitted; service that starts by the deadline but finishes after it is counted late. On-time, late, unserved and rejected requests remain separate.

Aerial opportunities require:
- the request's allowed mode, service duration and payload to fit;
- unique capacity references and non-overlapping aircraft occupancy;
- a flight window with current evidence, weather `go`, permission `approved`;
- no maintenance/site-closed/no-go interval overlapping the mission and turnaround occupancy;
- aerial opportunity count no greater than the supplied upstream steady-state plus initial-inventory capacity envelope.

Unknown/denied/stale weather or permission blocks aerial service. A request may use fallback only if its declared mode permits it. The fixture includes unknown permission and weather no-go cases, a maintenance window, a late arrival, a late completion and a request with no available opportunity.

The legacy fixture contains 11 requests, six manually declared aerial opportunities, six fallback opportunities and five evidence-bound flight windows. The module contains 26 Rust test definitions across legacy compatibility, true FCFS/EDF semantics and the linked receipt consumer for FCFS/EDF priority, fallback under unknown permission, stale/future evidence, maintenance, acceptance cutoffs, overlap, source-capacity budgets, deadline expiry, deterministic input permutations, mode constraints, payload capacity and late completion. Exact-head Actions run [#45134172381](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45134172381) passed schema/fixture parsing, workspace formatting, Cargo tests, deterministic report assertions and artifact upload (203 tests passed, 0 failed across test binaries). Clippy remains unverified.

The research dossier is `docs/research/SUSTAINABLE_MOBILITY_SERVICE_QUALITY_BENCHMARK_V0_1.md`. It compares the request-admission work to Zhao et al. (dynamic arrivals and changing weather, TR-E 2026, DOI 10.1016/j.tre.2026.105074), Lu et al. (real-options mode switching, TR-C 2026, DOI 10.1016/j.trc.2026.105807), Liang et al. (multi-visit pickup/delivery and drone resupply, EJOR 2026, DOI 10.1016/j.ejor.2026.10.020), and Xu et al. (mobile swapping/charging under uncertainty, TR-D 2026, DOI 10.1016/j.trd.2026.105458).

This remains a synthetic online admission model. The legacy compatibility mode still accepts manually declared opportunities; the preferred linked mode re-runs the capacity event evaluator and derives aerial schedule fields from the exact event receipt sequence. Payload/weather/permission envelopes remain declarative and unauthenticated. No route, flight, dispatch, market, regulatory or customer-delivery authority is granted.


## MOBILITY-CAPACITY-OPPORTUNITY-RECEIPTS-001 — event-derived aerial service opportunities

The composition child of [#6909](https://github.com/Luminous-Dynamics/symtropy/issues/6909) and [#6921](https://github.com/Luminous-Dynamics/symtropy/issues/6921) is tracked in [#6999](https://github.com/Luminous-Dynamics/symtropy/issues/6999).

The capacity event simulator now emits one deterministic `CapacityServiceOpportunity` for every mission that it scheduled and completed within the declared horizon. Each record includes:
- stable opportunity ID (`aircraft-NN-mission-NNNNNN`);
- exact aircraft ID;
- zero-based per-aircraft mission sequence;
- mission start time;
- mission duration;
- landing-to-next-takeoff turnaround.

The record is created only when the simulator initially schedules an aircraft's first mission, when an actual ready spare is assigned after landing, or when a completed charge restores a waiting aircraft. A charging queue that never completes cannot create a later opportunity. Missions that cannot finish inside the horizon are not exported as service opportunities.

The report publishes a canonical `service_opportunity_digest` (SHA-256 over the serialized ordered opportunity array). Records are sorted by start time, aircraft ID and mission sequence. CI asserts that receipt count equals completed sorties, checks the digest shape and asserts the frozen Dock-shaped profile's first opportunity ID.

The next adapter step is to make #6921 consume these exact upstream records, rather than accepting user-authored aircraft/time/duration slots. The digest detects inconsistency against the supplied report bytes; it is not an authenticated producer signature or proof of field performance.

Runner verification is pending on the current branch. Do not treat source-level receipt generation as verified until exact-head formatting, Rust tests, deterministic report assertions and artifact upload pass.


### Linked capacity-receipt mode — #6999

The linked input deliberately contains **only** each aerial opportunity ID, maximum qualified payload and an aircraft/configuration-bound payload evidence envelope. It does not accept aircraft IDs, mission start times, mission duration or turnaround values.

The CLI re-runs the #6909 capacity event evaluator on the second input, selects the exact architecture, verifies each emitted mission sequence and turnaround relationship, recomputes the canonical receipt digest and resolves every qualification ID to one source receipt. It derives the aerial service schedule from that receipt sequence, then evaluates FCFS and EDF on the same requests, weather/permission windows, maintenance blocks and fallback opportunities.

Payload evidence must name the exact capacity opportunity ID, aircraft ID, site ID, site configuration digest and SHA-256 architecture-configuration digest; it must be observed no later than the mission start and remain valid through mission plus turnaround. Replayed evidence references, duplicate or unknown receipt IDs, gaps in per-aircraft sequence, stale configuration, source-horizon mismatch and overlapping source receipts are rejected. The capacity report is generated internally from the architecture input rather than accepted as a user-authored schedule or trusted report digest.

The linked evaluator's new synthetic fixture references seven out of 38 event-derived opportunities from the one-aircraft Dock-shaped capacity profile. It compares the true arrival-ordered FCFS policy (deadline-independent ties) with EDF, while weather no-go, maintenance, unknown permission, fallback, a late completion and unavailable capacity remain explicit outcomes.

Strict schema: `schemas/sustainable_mobility_service_quality_linked_v0_1.schema.json`; fixture: `fixtures/sustainable_mobility_service_quality_linked_v0_1.json`; source capacity input: `fixtures/sustainable_mobility_service_capacity_v0_1.json`.

The linked report carries the derived capacity-report digest, opportunity digest and configuration digest. This proves internal consistency between the supplied architecture input and the event model's deterministic outputs; it does not authenticate the producer or turn synthetic evidence into physical truth. No flight, dispatch, customer delivery, regulatory or market authority is granted.

Research dossier: `docs/research/MOBILITY_CAPACITY_OPPORTUNITY_RECEIPT_CONSUMPTION_BENCHMARK_V0_1.md`.

## Linked receipt consumer — composition correction for #6999

The preferred linked CLI mode now takes two files: the linked request fixture and the versioned capacity architecture input. The request input contains receipt IDs and payload qualification only. The consumer re-runs the capacity evaluator and derives each aerial aircraft/time/duration/turnaround tuple from the resulting event receipts. It rejects unknown/duplicate IDs, per-aircraft sequence gaps/overlaps, mismatched payload aircraft/configuration, expired payload evidence and horizon mismatch.

The legacy one-file mode is retained explicitly with the --legacy flag for backward-compatible reproduction; it is not the preferred way to create new request fixtures. The new exact-schema fixture has twelve requests and seven qualified IDs from a 38-opportunity capacity report. Expected synthetic outcomes: FCFS 6 on-time / 2 late / 3 unserved / 1 rejected; EDF 7 on-time / 1 late / 3 unserved / 1 rejected. The policy difference is a real arrival-order FCFS (deadline-independent ties) versus EDF.

New dossier: docs/research/MOBILITY_CAPACITY_OPPORTUNITY_RECEIPT_CONSUMPTION_BENCHMARK_V0_1.md. Exact-head CI for this linked-consumer branch is pending; the source currently contains 24 focused tests, but no compile/test claim is made until the new workflow run completes.


## Linked service quality: exact assignment oracle and horizon accounting

The linked service-quality evaluator now distinguishes **completed on-time**, **completed late**, **in progress at horizon**, **unserved**, and **rejected after acceptance cutoff**. A service that starts but ends beyond the evaluation horizon has no service_completion_seconds or completed latency. Its projected completion is reported separately and it is not counted as delivered, late-completed or completed-requests/hour.

Run the frozen bounded exact-oracle challenge:

\`\`\`bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin sustainable_mobility_service_quality_screen -- \\
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_quality_exact_oracle_v0_1.json \\
  tools/ev-passenger-cost-frontier/fixtures/sustainable_mobility_service_capacity_v0_1.json
\`\`\`

The exact reference uses bitmask dynamic programming, bounded to at most 12 requests and 16 combined aerial/fallback opportunities, with a 1,000,000-transition ceiling. It is explicitly a clairvoyant full-horizon offline upper bound that sees the complete frozen request stream; it is not a deployable online policy. Above the input bounds it returns not_run_bounds_exceeded; on transition exhaustion it returns transition_limit_reached. Neither status includes an optimum or policy regret. For an evaluated instance, the objective is lexicographic: maximize on-time completions; maximize total completed-by-horizon services; minimize cumulative lateness; minimize work left in progress at the cut-off; minimize completed-request latency; then apply a canonical ID tie-break. No arbitrary weighted scalar score or service-class weight is introduced.

The two-request/two-opportunity frozen source-receipt challenge yields:
- FCFS: one on-time and one late completion; 185 seconds cumulative deadline lateness.
- EDF: two on-time completions; zero deadline lateness.
- Exact oracle: same outcome as EDF on this tiny instance; FCFS loses one on-time completion and has 185 seconds of lateness excess.

The full 12-request linked fixture has 7 qualified aerial plus 5 fallback opportunities and now falls within the 12-request/16-opportunity DP bound. It solves in 768 transitions: 7 on-time + 1 late, 8 completed, 0 in progress, 170 s cumulative lateness and 6,715 s completed latency. FCFS offline upper-bound gap is one on-time completion and 185 s excess lateness; EDF matches the upper bound on this fixture. Larger instances still abstain above declared bounds. Use the gaps as offline upper-bound distance, not online-policy regret.

The linked consumer still re-runs the capacity event simulator and derives aircraft/time/duration/turnaround only from source opportunity receipts. Exact oracle code: src/sustainable_mobility_service_quality.rs; bounded fixture: fixtures/sustainable_mobility_service_quality_exact_oracle_v0_1.json; full linked fixture: fixtures/sustainable_mobility_service_quality_linked_v0_1.json; dossier: docs/research/SUSTAINABLE_MOBILITY_EXACT_ORACLE_AND_SERVICE_HORIZON_V0_1.md.

Exact-head verification for the 12×16 bitmask-DP plus explicit offline-information-model revision: [Actions #45211825272](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45211825272) at `dc69e4e28a62cda46ca0198ecdf2e11d5d6e0756` passed schema/fixture parsing, workspace formatting, Cargo tests, deterministic legacy/linked/full-exact/tiny-exact/offline-model assertions and artifact upload. Test binaries reported 224 passed, 0 failed across binaries; the service-quality module contains 31 focused Rust test definitions. The full 12-request fixture solved in 768 transitions and returned 7 on-time, 1 late, 8 completed, 0 in-progress; 170 s cumulative lateness and 6,715 s completed latency. FCFS offline upper-bound gap vector was [1,0,185,0,0]; EDF [0,0,0,0,0]. The report explicitly labels the exact result a clairvoyant full-horizon offline upper bound and says deployable_online_policy=false. Clippy and physical/field validation are not included.


**Information boundary:** a full-horizon exact assignment can use later revealed requests when choosing earlier opportunity assignments. The result is therefore labelled `clairvoyant_full_horizon_offline_upper_bound` and `deployable_online_policy=false`. A separate nonanticipative scenario-tree oracle is tracked in [#7133](https://github.com/Luminous-Dynamics/symtropy/issues/7133).


## SERVICE-PREDICTION-CALIBRATION-001 — chronological calibration and deployment-shift audit

Tracked by [#7240](https://github.com/Luminous-Dynamics/symtropy/issues/7240). Research protocol: [Service Prediction Calibration and Deployment-Shift Benchmark v0.1](../../docs/research/SERVICE_PREDICTION_CALIBRATION_AND_DEPLOYMENT_SHIFT_BENCHMARK_V0_1.md).

Run:

```bash
cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin service_prediction_calibration_screen -- \
  tools/ev-passenger-cost-frontier/fixtures/service_prediction_calibration_v0_1.json
```

The evaluator separates a chronological calibration cohort from a later holdout and groups metrics by model/version, site configuration, operating stratum, exact resource configuration, metric and measurement boundary. It performs a finite-sample split-conformal interval expansion using calibration residuals only, then reports pre/post holdout coverage, Wilson 95% coverage bounds, interval width, MAE and Winkler score. On-time probability calibration is evaluated separately on holdout with Brier score and reliability bins.

The evidence contract binds every forecast and outcome to exact site/asset/configuration, producer, method, unique evidence reference and time validity. Forecasts must precede the target interval; outcomes must follow it and remain before the audit cutoff. Calibration outcomes must be observed strictly before the earliest holdout forecast in the same group. Pack-terminal Wh, charger-input Wh and PCC-meter values cannot be interchanged.

The frozen synthetic fixture has 100 records (40 calibration, 60 holdout), two strata (`nominal`, `hot_shift`) and 300 metric observations across mission duration, pack-terminal energy and PCC peak import. Expected post-calibration coverage is 1000 permille for each nominal group, and 0 permille with an undercoverage alarm for each hot-shift group. The hot-shift on-time outcome rate is 600 permille despite a constant 900-permille predicted probability.

These are synthetic controls. The evaluator explicitly sets field qualification, model mutation and operational authority to false. Its report labels Wilson intervals as an IID reference calculation; temporal/common-cause dependence needs cluster-aware inference. Synthetic evidence cannot produce an empirically supported status. Split conformal is reported as an empirical diagnostic only; no distribution-free guarantee under arbitrary temporal dependence or deployment shift is claimed.

Files: `src/service_prediction_calibration.rs`, `src/bin/service_prediction_calibration_screen.rs`, `schemas/service_prediction_calibration_v0_1.schema.json`, `fixtures/service_prediction_calibration_v0_1.json`. The module currently contains 17 focused Rust test definitions. Exact-head Actions run [#45333671953](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45333671953) at `9f61b00ea3b4077f5f67409222e27dc20806f835` passed schema/fixture parsing, workspace formatting, Cargo tests, known-answer report assertions and artifact upload (241 tests passed, 0 failed across test binaries). Clippy and field/physical qualification remain separate.

## ONLINE-SERVICE-RISK-CERTIFICATE-001 — prequential temporal risk monitor

Tracked by [#7289](https://github.com/Luminous-Dynamics/symtropy/issues/7289), child of [#7240](https://github.com/Luminous-Dynamics/symtropy/issues/7240). Research protocol: [Online Service Risk Certificate and Temporal Shift Monitor v0.1](../../docs/research/ONLINE_SERVICE_RISK_CERTIFICATE_BENCHMARK_V0_1.md).

Run:

    cargo run --quiet --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml --bin online_service_risk_screen -- tools/ev-passenger-cost-frontier/fixtures/online_service_risk_v0_1.json

The evaluator groups exact site, asset/configuration, model/version, stratum, metric and measurement boundary. At each forecast issuance it consumes only outcomes whose evidence became available strictly earlier. It maintains a time-uniform one-sided Hoeffding-Azuma upper bound for average predictable historical interval-miscoverage under bounded martingale-difference assumptions and a separate one-sided CUSUM drift alarm. Outcomes can arrive out of forecast-issue order; the pending queue is ordered by actual availability.

The confidence bound is not a bound on the next conditional error rate or arbitrary future shift. CUSUM is a deterministic alarm, not a probability guarantee. New asset configurations start with no inherited history. The module never updates a model or grants field/operational authority, and all existing reserve/recovery/lease/PCC/network gates remain authoritative.

The five-observation synthetic fixture expects two interval misses, prior-history counts [0,0,0,2,3], one latched shift alarm and final empirical miscoverage of 400 permille. Thirteen focused Rust tests cover delayed/out-of-order outcomes, no future leakage, CUSUM latching, configuration isolation, deterministic replay, evidence chronology, duplicate references, interval endpoints and claim ceilings.

Files: src/online_service_risk.rs, src/bin/online_service_risk_screen.rs, schemas/online_service_risk_v0_1.schema.json, fixtures/online_service_risk_v0_1.json. All fixture evidence is synthetic; no physical or field qualification is implied.
