# SAFRB time-expanded adapters

This crate contains two deliberately distinct, research-only mappings into the shared single-commodity flow kernel.

## Existing SAFRB obligation adapter

The original adapter maps the frozen synthetic SAFRB v0.1 obligation profile into the shared flow kernel.

It validates exact node/edge identities, one-step time chronology, integer lower/upper capacity bounds, one quantity unit, opening-stock reconciliation, route aliases to shared physical-resource capacity gates, matching per-window capacity evidence, and service obligations at beneficiary nodes.

It builds separate lower and upper graph instances. It reports (1) joint all-obligation feasibility, (2) each beneficiary window's individual maximum under lower and upper bounds, and (3) the allocation in one joint flow witness. Independent maxima are not additive when obligations share capacity and are never presented as a jointly feasible schedule. Joint lower/upper results and every computed isolated maximum include portable `FlowCertificateBundle` objects that carry the exact modified graph, graph digest, edge-flow witness and min-cut. Consumers must independently call `bundle.verify()` after deserialization. Hypothetical restoration results also carry lower/upper bundles, but remain explicitly `hypothetical_counterfactual_not_admitted`. To bound solver cost, they are computed only when the joint lower-bound problem is infeasible and there are at most 32 obligations; they are not computed when joint lower-bound service is already feasible, are explicitly omitted above that limit, and are skipped during counterfactual search. The v0.1 profile supports at most 128 obligations and one beneficiary obligation per time index. It also binary-searches a monotone minimum hypothetical capacity uplift and marks changed capacity evidence as hypothetical.

The generic kernel owns only graph mathematics. Mycelix owns identity, issuer recognition, legal access, evidence lineage, privacy and common-cause dependency closure. Symthaea owns decision-aware evidence acquisition. Sol-Atlas owns read-only projection. Power Fabric owns the beneficiary service profile. This adapter does not authenticate measurements, establish legal rights, convert units, qualify field capacity, estimate prices, authorize action, or prove actual beneficiary service.

A successful lower-envelope result means only that the declared synthetic graph can satisfy all listed obligations simultaneously. An individual per-window maximum is a separate diagnostic and does not prove joint feasibility or a beneficiary guarantee. A failed or malformed input returns an error; unsupported schema/model forms are not coerced into a single-commodity graph.

## Temporal inventory/handling profile v0.1

The separate `temporal_profile` module explores a narrower and explicit temporal inventory model. It is versioned independently from the legacy adapter schema:

`safrb-temporal-inventory-handling-arrival-at-period-start-v0.1`

The model separates four distinct constraints:

1. **End-of-period storage carry.** Inventory retained between periods traverses a carry edge capped by node storage capacity.
2. **Outbound handling.** Every transfer departure from the same physical node during the same period passes through one shared handling gate.
3. **Beneficiary service issue.** Service withdrawals at a node/period pass through a separate gate, so issue capacity is not conflated with outbound handling.
4. **Route capacity and transit.** Each declared transfer has its own route capacity and explicit whole-period transit. Zero-transit and same-period movement are rejected.

The period-start convention is explicit: opening inventory and receipts become available at the start of their declared period. This permits same-period cross-docking when flow is routed through the handling gate; only inventory carried beyond the period is subject to the storage carry capacity. This profile does not model peak in-period occupancy, unloading, staging, or a forced dwell interval. Those require additional constraints and are not inferred.

For physical node (n) and period (t), the compiled graph enforces the inventory balance

[
O_{n,t} + R_{n,t} + A_{n,t} + C_{n,t-1}
=
H_{n,t} + Q_{n,t} + C_{n,t},
]

where (O) is opening stock (period zero only), (R) is period-start receipts, (A) is transfer arrivals, (C) is carried inventory, (H) is outbound transfer handling, and (Q) is service issue. The gates enforce:

- (0 \le C_{n,t} \le S_n) for periods with a following period;
- (H_{n,t} \le H_n^{\max}) across all departures from node (n) in period (t);
- (Q_{n,t} \le Q_n^{\max}) across all services at node (n) in period (t);
- each transfer flow is bounded by that transfer's route capacity and arrives only after its declared transit periods;
- each service edge is capped by its `requested_quantity`.

`requested_quantity` is a service request/ceiling, **not** a mandatory minimum-delivery constraint. A maximum-flow result states how much of the declared request can be served in this graph; it does not by itself prove a minimum service obligation was met.

For v0.1, storage, handling and service-issue capacities are constant over the modeled horizon. Dynamic per-period capacities, common physical resources shared across distinct node identities, multiple commodities, grade conversion, process coupling, stochastic disruptions and intra-period event ordering are outside this model class.

Each profile includes a case identifier and a period-timebase reference. Both, along with the quantity unit and beneficiary boundary, are bound into canonical graph identity. The reference is not dereferenced or authenticated by this crate: callers must qualify its issuer, authority, validity window and source data outside this compiler before treating it as evidence.

### Independent temporal certificate verification

The separate `temporal_verify` module first verifies the kernel's graph-bound max-flow/min-cut bundle, then independently reconstructs the expected temporal graph from the supplied profile. It does not call the temporal compiler or solver. It checks:

- exact graph node and edge identities, endpoints, capacities and metadata;
- case, unit, quantity-boundary and period-timebase binding;
- complete edge-flow witness coverage and per-edge bounds;
- per-node/per-period inventory conservation;
- reconciliation of every handling gate to all transfer departures;
- reconciliation of every issue gate to all service withdrawals;
- equality of source receipts/opening-stock flow, beneficiary service flow and certified maximum flow.

A certificate is only meaningful together with the exact profile used to interpret it. Changing a receipt quantity, case identifier or timebase makes the prior bundle invalid for that profile.

### Tests and qualification

The temporal tests cover cross-dock throughput above end-of-period storage, carried inventory bounded by storage, shared handling across parallel departures, route arrival ordering, zero-transit rejection, opening-stock drawdown, overflow and expansion-budget checks, deterministic graph encoding, and mutations of receipt capacity and timebase. A 250-seed bounded multi-beneficiary corpus is checked against an exhaustive physical-quantity oracle that does not rebuild the compiled graph; every positive result also passes the independent temporal verifier. The corpus tests aggregate requested delivery only, not mandatory beneficiary floors or priority allocation. Existing adapter tests cover shared-pump aliases, stale evidence, half-open time intervals, and joint-versus-individual maxima.

The Rust exact-head workflow is `.github/workflows/safrb-flow-rust.yml`. It pins the repository Rust toolchain and checks exact checkout, formatting, Clippy, Rust tests, and independent portable certificate conformance. A green run qualifies only the exact commit recorded by that run; later commits require fresh receipts. Repository tests do not establish evidence authenticity, legal access, physical asset qualification, dispatch authority, or actual beneficiary service.

## Beneficiary minimum-floor feasibility v0.1

The separate `beneficiary_floors` module proves whether a set of mandatory per-beneficiary minimum quantities can be met **jointly** in the supplied temporal single-commodity graph. It does not treat the graph's aggregate maximum requested delivery as a substitute for a floor-satisfying allocation, and it does not optimize priority tiers or operational cost.

### Exact lower-bound circulation reduction

For each declared service floor, the service edge receives a lower bound `l` and retains its request ceiling as upper bound `u`. The producer replaces each original capacity by `u - l`, applies the node-balance transformation, adds a sink-to-source return arc and super-source/super-sink arcs, and solves the resulting circulation-feasibility max-flow. All balances and capacity sums use checked arithmetic; graph expansion is checked against kernel node/edge ceilings before solving.

- If the auxiliary maximum flow equals the total required super-source flow, every mandatory floor is jointly feasible. The report includes the recovered original edge-flow allocation and per-beneficiary reconciliation.
- If the auxiliary maximum flow is smaller, the lower-bound circulation cannot be satisfied. The report retains the exact auxiliary graph-bound max-flow/min-cut certificate and exposes **no original allocation**.

### Independent verifier

The verifier reconstructs the auxiliary graph from the original temporal graph and submitted floors without calling the producer's reduction builder. It compares canonical graph bytes, verifies the kernel certificate, checks that the report's feasibility disposition matches the max-flow result, reconstructs the original flow, verifies node conservation and checks every beneficiary floor against its request ceiling. The report binds the exact base-graph digest, case ID, obligations and auxiliary graph.

### Portable contracts

- `schemas/beneficiary-floor-feasibility-input-v0.1.schema.json` defines the temporal profile plus mandatory-floor request envelope.
- `schemas/temporal-profile-v0.1.schema.json` defines the strict temporal profile input used by both scenario and floor requests.
- `schemas/beneficiary-floor-feasibility-v0.1.schema.json` defines the report, exact base graph, auxiliary certificate and optional allocation.
- Rust types use strict Serde serialization/deserialization and reject unknown fields.
- Tests cover joint feasibility, jointly impossible floors despite individual route possibility, request-ceiling violations, unknown/duplicate services, changed floors, forged allocations, strict JSON round-trip and schema-document parsing. A 250-seed bounded multi-beneficiary corpus checks the reduction against an independent physical-quantity oracle built from per-beneficiary route/issue/request limits and shared supply/handling limits; every result is then checked by the separate report verifier.

### Claim boundary

This is a synthetic joint-feasibility proof for the declared graph only. It does not prove that the beneficiary identity is authenticated, that the physical capacity evidence is true or current, that legal authority exists, that a dispatch is authorized, or that service was actually delivered. Priority allocation and cost optimization are separate stages and must preserve all mandatory floors; no arbitrary Big-M scalarization is admitted.

## Robust beneficiary-floor scenarios v0.1

The separate `robust_floor_scenarios` module evaluates one common set of mandatory beneficiary floors in every supplied coherent scenario. It composes with `beneficiary_floors`: each scenario gets its own exact lower-bound circulation certificate and independently verified original allocation or infeasibility result.

### Disposition semantics

- `GuaranteedAcrossSuppliedScenarios`: every supplied scenario jointly satisfies every declared floor.
- `PossibleInAtLeastOneSuppliedScenario`: at least one supplied scenario satisfies all floors, but at least one does not; this is not a robust guarantee.
- `NotFeasibleInAnySuppliedScenario`: no supplied scenario satisfies all floors.

All scenarios must share the same commodity, unit, beneficiary quantity boundary, timebase, temporal schema and horizon. Every scenario must declare every floor service with a request ceiling at least as high as the floor. Incomparable inputs are rejected rather than reported as a range.

### Independent verification and portable identity

The report retains each per-scenario floor report and certificate. Its independent verifier recomputes the scenario-set digest, floor-set digest and combined assurance-input digest; verifies every scenario-specific lower-bound certificate; and recomputes the aggregate disposition and counts. Scenario and profile-array ordering does not change canonical identity, while changing a scenario profile, floor or scenario-set ID invalidates the old report.

Portable schemas:
- `schemas/robust-beneficiary-floors-input-v0.1.schema.json`
- `schemas/robust-beneficiary-floors-report-v0.1.schema.json`

### Claim boundary

Guaranteed means guaranteed across the *supplied finite synthetic scenario set only*. It does not establish that the set is complete, that scenarios are authenticated or mutually exclusive in the physical world, that probabilities are known, or that source authority, legal rights, physical qualification, dispatch authorization or actual service exists. It never splices capacity maxima from different scenarios.

## Next research increments

- [#7735](https://github.com/Luminous-Dynamics/symtropy/issues/7735): temporal semantics and the distinction between carried inventory and throughput.
- [#7751](https://github.com/Luminous-Dynamics/symtropy/issues/7751): dynamic evidence-bound capacity windows, receipt admission modes, shared cross-node resources, and finer event ordering.
- [#7710](https://github.com/Luminous-Dynamics/symtropy/issues/7710): independent min-cost-flow optimality certificates and LEMON/OR-Tools comparison.
- [#7760](https://github.com/Luminous-Dynamics/symtropy/issues/7760): explicit beneficiary minimum floors, priority tiers and independent allocation feasibility.
- [#7647](https://github.com/Luminous-Dynamics/symtropy/issues/7647): scale and resource-budget qualification.

The solver comparison must use equivalent objectives, graph models, integer ranges, units, hardware and budgets. DIMACS Network Flows 2.0 instances and generators provide a shared basis; LEMON CostScaling/NetworkSimplex and OR-Tools are solver comparators, not a substitute for semantic validation. No SOTA performance advantage is claimed by this adapter or its current small conformance corpus.
