# Maritime Exact Resource Oracle v0.1

**Status:** synthetic exhaustive micro-instance; dedicated exact-head workflow passed.  
**Tracking:** [#7519 — MARITIME-EXACT-ORACLE-001](https://github.com/Luminous-Dynamics/symtropy/issues/7519).  
**Related:** #3683, #7231, #7357, #7504, #7510, #7512.

## Purpose

This crate is an exact, dependency-free reference for tiny time-indexed resource allocation problems. It complements MDTR:

- MDTR asks whether the information and decision were justified at the historical cutoff.
- This oracle asks whether a combination of candidate actions fits the declared physical resource capacities and which feasible combinations lie on the Pareto frontier.

It is not a production scheduler, terminal simulator or continuous-time physics model.

## Shared-resource contract

A candidate action belongs to exactly one vessel. It reserves integer units of resources at discrete time slots. A scenario supplies a capacity for each (resource, slot) pair. The exhaustive solver selects exactly one candidate per declared vessel.

The same model represents berth occupancy, quay/yard cranes, AGV fleet, approach bridge, yard slots, tug/pilot windows, onshore-power sockets, grid capacity, rail/gate/inland slots and tide availability. A capacity of zero closes a resource at that time. Reservations are conjunctive; objective improvements never compensate for an over-capacity plan.

## Objective vector

Each candidate contributes five separately reported integer metrics:
1. vessel waiting;
2. end-to-end receiver delay;
3. energy cost;
4. emissions;
5. missed connections.

A plan dominates another only when it is no worse on every axis and strictly better on at least one. Equal or incomparable outcomes remain on the frontier. No arbitrary scalar weights are introduced.

## Current tiny port instance

The frozen fixture has two vessels, six candidate actions and time-indexed capacities for:
- two berths, including a tide-closed slot;
- a shared quay-crane pool;
- AGV fleet;
- constrained bridge corridor;
- yard block;
- shore-power socket.

Exhaustive enumeration should produce four feasible combinations and three Pareto-undominated plans. The tide-closed candidate and early conflicting assignments are excluded by hard capacity checks. The hand-authored output is independently reproduced by a standard-library Python implementation.

## Run locally

From the repository root:

```sh
cargo fmt --manifest-path research/maritime-exact-resource-oracle/Cargo.toml -- --check
cargo test --locked --manifest-path research/maritime-exact-resource-oracle/Cargo.toml
cargo clippy --locked --manifest-path research/maritime-exact-resource-oracle/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path research/maritime-exact-resource-oracle/Cargo.toml --bin resource-oracle-audit > resource-oracle-receipt.tsv
python3 research/maritime-exact-resource-oracle/scripts/independent_reference.py --receipt resource-oracle-receipt.tsv
```

## Why this is useful for the SOTA tournament

Optimization systems should not be compared only by their own feasibility reports. Each candidate generator (FCFS, rolling horizon, MILP/CP-SAT, robust C&CG, ALNS/RL-ALNS, GNN/MAPPO, HA-QMIX or branch-and-price) should emit the same candidate-action interface. This tiny oracle can exactly adjudicate feasible combinations and Pareto dominance on enumerable cases.

For larger instances, use a solver-provided valid bound and publish timeouts / incumbents. Do not claim an optimality gap when no valid bound exists. The exhaustive oracle is a ground-truth reference only for its declared finite discrete model.

## Limitations and claim ceiling

- Candidate actions are supplied; this crate does not generate an optimization policy.
- Time is discrete; continuous berth positions, crane travel curves, collision geometry, stochastic arrivals and tide hydrodynamics are not modeled yet.
- Resource identities/capacities are supplied by the scenario; the oracle does not prove that they match a real port.
- Objective values are fixture inputs, not independently measured outcomes.
- No cargo custody, evidence rights, authority, receiver acceptance, operational control or live terminal source is connected.
- The fixture is synthetic; a passing result is not real-port performance, global generalization or a SOTA win.

Next increment: add a second topology family with bridge/tide windows and then compare typed candidate outputs from the optimizer families against exact feasible sets and Pareto fronts. Keep physical/authority qualification with the existing #7231/#7357 owners and decision-time evidence with #7505.


## Receipt completeness

The deterministic receipt now emits the canonicalized scenario manifest before the feasible set:
- sorted vessel identities;
- every time-indexed resource capacity, including zero-capacity tide closures;
- every candidate ID, vessel binding and objective vector;
- every candidate reservation by resource and slot;
- complete feasible combination set and Pareto frontier.

The independent Python reference reconstructs the same canonical manifest directly from the fixture, exhaustively enumerates the candidate Cartesian product, computes the Pareto set independently, and compares the entire byte-level receipt. This makes a solver result auditable against the exact capacity and reservation inputs rather than reporting only plan IDs and objective values.


## Sea–rail topology regression

The Rust unit suite now includes a second hand-authored topology family that reserves rail tracks, rail-mounted gantry-crane capacity and receiver-side rail-gate slots. Two vessel choices that each look feasible in isolation cannot both consume the same one-capacity track/time slot. The exact enumerator retains the two cross-track combinations, and both remain Pareto-undominated because they trade energy cost against emissions.

This test is a resource-model regression, not a second independently serialized fixture yet. The primary 2-vessel berth/bridge/AGV/yard/OPS fixture remains the separately checked receipt (4 feasible / 3 Pareto); the sea–rail case is checked in Rust tests only.


## Tidal mooring + approach bridge regression

A new exhaustive two-vessel unit test adds the core topology coupling found in Pang et al., *Transportation Research Part B* 213 (November 2026), DOI 10.1016/j.trb.2026.103584:

- mutually exclusive berth-segment reservations for north/south mooring orientation;
- tide-window capacity, including a closed slot represented by capacity zero;
- shared approach-bridge corridor reservations by time slot.

The test enumerates only the two cross-orientation assignments. Same-orientation pairs overbook a berth segment and bridge; the tide-closed candidate is impossible. This is still a discrete micro-regression, not a reproduction of the paper's continuous tide/channel-time functions, berth geometry or complete Ningbo-Zhoushan dataset. The paper reports up to 10.9% objective improvement over the best Gurobi result within a two-hour limit on its own large instances; that percentage is literature context, not a result from this crate.


## South African DGT recovery micro-regression

A synthetic two-vessel test now encodes the public intervention pattern described in Transnet's November 2026 DGT communications:
- Pier 2 is modeled as unavailable in the tested slot;
- direct geared-vessel discharge uses DGT Berth 102 and its compatible discharge capacity;
- an alternative diverts via Ngqura berth/QC capacity, a feeder-transfer slot, a later Berth 102 arrival and Gauteng rail evacuation;
- berth, feeder and rail slots are shared resources, so both vessels cannot consume the same single-capacity direct/diversion route simultaneously.

The exact instance admits two cross-route combinations (one direct and one Ngqura-feeder) and rejects the two Pier 2 options. This is a synthetic resource-capacity translation of public intervention descriptions, not a reconstruction of actual DGT schedules, container counts or operational availability.

Public context:
- Transnet, 18 November 2026: https://www.transnet.net/getFile.ashx?id=34389
- Business Day, 25 November 2026: https://www.businessday.co.za/economy/2026-11-25-durban-port-backlog-to-last-into-january-transnet-says/

The additional Rust unit regressions cover sea–rail tracks and tide/mooring/bridge coupling. DGT direct-vs-diversion has now also been promoted to a separately frozen TSV receipt and independent reference.


## Independently reproduced DGT recovery case

The DGT direct-discharge vs Ngqura-feeder case has been promoted from a Rust-only regression into its own frozen TSV scenario and deterministic executable. The receipt is checked by a separate standard-library Python Cartesian enumerator.

The expected synthetic result is:
- exactly two feasible combinations: one vessel uses direct geared discharge at DGT Berth 102, the other diverts via Ngqura and feeder transfer; or the reverse;
- both alternatives remain on the Pareto frontier because the fixture gives them equal aggregate objective vectors;
- both Pier 2 candidates are excluded by zero-capacity availability;
- attempting to use both direct berth slots, both Ngqura/feeder slots or a shared constrained rail slot exceeds declared capacity.

This scenario is a discrete test of the route/resource logic inspired by public DGT recovery interventions. It is not reconstructed from Transnet vessel/cargo records and it does not measure actual berth 102, NCT, feeder or rail availability.

The dedicated workflow now emits and checks two receipts:
1. the primary micro-port receipt (4 feasible, 3 Pareto);
2. the DGT recovery receipt (2 feasible, 2 Pareto).

The DGT run is independently reproduced from `fixtures/dgt-recovery.tsv` by `scripts/dgt_reference.py`. Both receipts are uploaded as workflow artifacts. The DGT fixture passed its exact-head independent receipt check; see the qualification record below.


## Exact-head qualification record — primary + DGT receipts

The dedicated `maritime-exact-resource-oracle` workflow passed on commit `5c9474ccba68dfd1cf0d51f95b8f057e04cbccc7`:
- Formatting: pass
- Rust unit tests: 13 passed, 0 failed
- Strict Clippy: pass
- Primary micro-port receipt: pass; independent reference reports 4 feasible / 3 Pareto
- DGT recovery receipt: pass; independent reference confirms 2/2 cross-route bundles
- Both deterministic receipts uploaded

The DGT fixture is a discrete synthetic translation of the public intervention pattern. It is not a Transnet schedule reconstruction or an independently measured capacity / performance claim. The sea–rail and tidal mooring/bridge additions remain unit-level regressions, while the primary and DGT fixtures have independent serialized receipt references.


## QC–AGV event synchronization under service-time uncertainty

The oracle now contains a separate `sync` module and a deterministic `qc-agv-sync-audit` receipt. It models the handoff event as:

`service_start = max(QC_ready, AGV_ready)`

For each task it reports QC waiting, AGV queue waiting, service finish and propagated completion. AGV route candidates are fixed before the service-time scenario; service-time deviations affect event timing, not route selection. A finite L1 uncertainty budget `Gamma` is enumerated exactly over `u_i ∈ {-1, 0, +1}` with `sum |u_i| <= Gamma`. The scenario vector is keyed by sorted task identity, so the same realization means the same task deviation across different candidate routes.

The frozen QC–AGV scenario demonstrates a nominal/robust route switch with explicit QC availability and asymmetric waiting weights:
- task A is QC-ready at time 0; task B is QC-ready at time 4;
- QC idle waiting is weighted 5 and AGV queue waiting is weighted 1;
- nominal route AB costs 45; nominal route BA costs 44, so BA is preferred nominally;
- under `Gamma=1`, route AB has worst-case cost 50 (witness `[A:+1, B:0]`);
- route BA has worst-case cost 59 (witness `[A:0, B:+1]`), so robust selection switches to AB;
- the nominal route's cost is lower, while robust selection protects the bounded worst-case waiting objective.

This is an intentionally tiny, fixed-order, two-QC/two-task mechanism test—not a reproduction of Song et al.'s continuous-budget model or S6-B. The uncertainty set is discrete and finite, not a continuous-budget guarantee. No general improvement is claimed.

The exact-head workflow now checks three receipts:
1. primary resource microinstance (4 feasible / 3 Pareto);
2. DGT recovery case (2 feasible / 2 Pareto);
3. QC–AGV synchronization (nominal best BA, robust best AB; five Gamma=1 scenarios).

Each receipt is recomputed by a separate standard-library Python reference. The sync reference reconstructs event readiness and waiting independently; this does not authenticate real QC service-time bounds or measure terminal receiver service.

## November 2026 QC–AGV robust-routing comparator

A new comparator must now be included: Song et al., *Journal of Marine Science and Engineering* 14(23), 2244 (25 November 2026), DOI [10.3390/jmse14232244](https://doi.org/10.3390/jmse14232244). The paper's core route model differs materially from the current two-task `sync` microcase.

### Correct comparator contract

- Fixed QC task orders; AGV vehicle assignments and successor routes are chosen.
- Handoff starts at the exact maximum of QC readiness and AGV arrival.
- Both QC idle and AGV queue waiting are costed separately.
- Loading AGVs are released at QC service start; unloading AGVs remain occupied through QC service and loaded delivery.
- Continuous two-sided service-time deviations use `p_i(u)=pbar_i+delta_i*u_i`, `-1 <= u_i <= 1`, and `sum_i |u_i| <= Gamma`.
- Route selection is here-and-now; event times are recomputed for every uncertain realization.
- A continuous separation problem supplies a feasible adverse witness and a valid upper bound; the master supplies a lower bound. A timeout or repeated adversary point without gap closure is not an optimality certificate.
- A sufficient loose-horizon condition is used in the paper. A failure of that sufficient condition is not proof of infeasibility; binding deadlines need separate robust feasibility checks.

The paper's two-task `sync` mechanism test in this repository is not a reproduction of that paper. Its uncertainty set is the finite discrete subset `{-1,0,1}` with an integer L1 budget.

### Published result: a deliberately narrow win, not general SOTA

For six-task synthetic instance S6-B at `Gamma=1`, the paper reports nominal route cost increasing from 69.1000 to 71.0833 CNY while the certified route-specific worst-case cost falls from 82.3000 to 77.0833 CNY. S6-B is the only improvement among its tested matrix; the article reports 16 certified conditions and eight unresolved. The result is a conditional synchronization mechanism under fixed crane orders, an instantaneous loading-release convention and a loose completion horizon—not an estimate of operating savings or evidence that robust rerouting generally wins.

The paper's text identifies S6-B local tasks 0–5 as source IDs 69, 90, 94, 175, 1034 and 1153; QC 1 handles tasks 1, 2, 4; QC 2 handles tasks 0, 5; QC 3 handles task 3. Tasks 1, 2 and 4 load; the others unload. Full durations, deviation amplitudes, yard blocks, distances and event records reside in the accompanying computational archive / are available from the authors. They have not been reconstructed here. Do not infer them from aggregate costs.

### Our next reproduction gate

Issue [#7602 — continuous-budget QC–AGV reproduction](https://github.com/Luminous-Dynamics/symtropy/issues/7602) defines the next work. Before comparing performance, we must:
1. implement a fixed-route continuous adversary with exact event timing and direction-dependent release;
2. provide a small independent LP/active-region oracle and continuous uncertainty witnesses;
3. recover or request exact S6-B source inputs before claiming reproduction;
4. compare nominal route, continuous worst costs, task-1 plus/minus 36 s witnesses, both routes under both witnesses, and certification bounds;
5. extend from the paper's deliberately narrow assumptions to finite yard buffers, traffic conflicts, charging, failures and receiver-service outcomes, as separate extensions.

No exact S6-B replication or SOTA gain is claimed by the current two-task fixture.