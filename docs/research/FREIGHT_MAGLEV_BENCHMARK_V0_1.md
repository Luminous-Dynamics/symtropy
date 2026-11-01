# Underground Freight, MagRail and Dedicated Maglev Benchmark v0.1

**Status:** research specification; synthetic-first; not a construction or technology-readiness claim
**Tracking:** [FREIGHT-MAGLEV-001 #3201](https://github.com/Luminous-Dynamics/symtropy/issues/3201)
**Parents / composition:** [TRANSPORT-OPT-001 #1601](https://github.com/Luminous-Dynamics/symtropy/issues/1601), [BROWNFIELD-UNDERGROUND-001 #1951](https://github.com/Luminous-Dynamics/symtropy/issues/1951), [MARITIME-REDUNDANCY-001 #3199](https://github.com/Luminous-Dynamics/symtropy/issues/3199)

## 1. Decision problem

Given a declared port-to-hinterland demand, determine which combination of existing infrastructure, targeted upgrade, new surface/underground construction, mode, terminals and operating policy delivers the best robust door-to-door service under safety, capacity, legal, environmental and community constraints.

Do not ask whether maglev is faster than rail in isolation. Ask whether the complete system is better after terminal dwell, transfers, empty returns, energy, construction, maintenance, rescue access, utilisation, disruption and demand uncertainty are included.

A canal being infeasible does not make underground maglev the default. The alternative set remains open and includes conventional rail, road, port operations, terminal handling, pipelines/conveyors for suitable commodities, partial tunnel links and no-build.

## 2. Technology taxonomy

| Family | What it is | What it is not |
|---|---|---|
| Conventional freight rail | Steel wheel/rail traction; existing or new right-of-way | Not automatically low-cost if the line, terminal or capacity is unavailable |
| Conventional rail tunnel | Conventional railway in an underground section | Not maglev; tunnelling does not change wheel/rail physics |
| Linear-motor / MagRail assistance | Linear motor provides propulsion or traction assistance; exact guidance/support architecture must be declared | Linear-motor propulsion alone does not prove magnetic levitation |
| Magnetic-levitation freight | Vehicle levitated/guided by a defined electromagnetic system, with explicit propulsion and control | Not a synonym for every linear motor, pod or magnetic guideway |
| Automated freight pods | Small or modular cargo carriers on a guideway; may use wheels, rollers, linear motors or other traction | Not necessarily high-speed or levitated |
| Conveyor / continuous material handling | Continuous transport for a suitable bulk commodity and route geometry | Not a general substitute for container rail |
| Slurry / pneumatic / pipeline | Commodity-specific fluid or gas transport with pumping/compression and processing | Not suitable for arbitrary freight; product handling and environmental constraints govern |
| Road / electric truck | Flexible collection, distribution and feeder movement | Not necessarily scalable or competitive for dense long-haul flows |
| Waterway / canal | Navigable water transport where depth, seasonality, lock, water and ecology constraints permit | Not a universal substitute when water is scarce or canals are inadmissible |

Each alternative must carry its own declared technology maturity, vendor/model source, interfaces, operating envelope, and engineering-validation need. Do not transfer a pilot result from one technology family to another.

## 3. Industry position and reference evidence

A 2025 systematic review of 243 underground-logistics publications identifies intermodal/port-container and other freight applications, but reports that many concepts remain unrealized because of high initial investment, limited policy support and insufficient demand. This is a reason to improve comparative screening and real-options analysis, not to dismiss the technology categorically. Hu et al., Tunnelling and Underground Space Technology 162 (2025), 106641. https://doi.org/10.1016/j.tust.2025.106641

A 2026 study applies two-stage robust optimization to underground container-logistics investment under carbon-tax and subsidy uncertainty. It is an optimization-method comparator, not evidence that a particular tunnel is viable. https://doi.org/10.1016/j.tust.2026.107444

In July 2025, DP World, Deendayal Port Authority and Nevomo announced an agreement to explore a proposed 750 m MagRail pilot within an existing port ecosystem. A January 2026 report describes a linear-induction-motor cargo-pod prototype test and a proposed later maglev step. Keep the tested LIM pod and proposed levitation demonstration distinct. https://www.dpworld.com/en/news/dp-world-deendayal-port-authority-and-nevomo-sign-mou-to-explore-autonomous-magnetic-rail-freight-movement-in-india ; https://indianinfrastructure.com/2026/01/20/deendayal-port-authority-tests-lim-pod-prototype-for-cargo-movement-sets-new-record-in-cargo-handling/

Research has also examined maglev-derived freight concepts intended to minimize modification to conventional rail corridors. This supports testing retrofit options as a design family, not presuming compatibility with existing track, vehicles or safety systems. https://doi.org/10.3390/machines12120863

## 4. Baseline-first alternative portfolio

For each origin/destination, use the same demand matrix, commodity mix, service window, terminal boundaries and price base. Include:

1. No-build / demand management / operational scheduling.
2. Existing rail timetable, terminal, signalling, locomotive, yard and maintenance improvement.
3. New conventional rail / grade separation / partial tunnel sections.
4. Linear-motor assistance or MagRail retrofit on an existing right-of-way, where technically compatible.
5. Dedicated freight maglev guideway, with surface and tunnel variants.
6. Automated freight pods on a conventional guideway or a smaller-diameter tunnel.
7. Commodity-specific conveyor, pipeline or slurry system only when the commodity is compatible.
8. Road/electric truck and road-rail intermodal options.
9. Waterway/canal only when the hydrological, ecological, water-demand and navigation gates are passed.

A modal alternative may be combined with rail/road feeders, but each transfer and interface is costed and capacity-limited. A single journey cannot count the same freight twice as it passes through multiple modes.

## 5. Engineering and operating envelope

All variables must have units, source class, validity period and uncertainty. Depending on the technology, model:

- throughput in tonnes/day, TEU/day or shipments/hour; cargo density, dimensions, payload, compatibility and commodity restrictions;
- route length, vertical profile, gradients, curve radii, clearance, structure gauge, tunnel diameter, shaft/portal locations and right-of-way;
- train/pod length, headway, dispatch policy, acceleration/braking profile, speed, dwell, transfer time, empty-return ratio and load factor;
- wheel/rail adhesion and wear, linear-motor efficiency, lift/guidance power, control system, auxiliary loads and regenerative braking, only where the technology requires them;
- terminal cranes/lifts, loading buffers, customs/security, cross-docking, first/last mile and service-window coordination;
- grid connection, peak power, storage, outage policy, black-start/backup assumptions and traction/levitation energy;
- construction quantities, tunnel boring, lining, spoil, shafts, stations, land/servitudes, utility relocation, construction staging, schedule and capex uncertainty;
- geology, groundwater, faulting, subsidence, drainage, flood ingress, heat, ventilation, smoke control, fire protection, emergency exits, cross-passages, rescue access and maintenance possession;
- staff, remote operations, cyber/control dependencies, spares, maintenance skills, recovery and end-of-life.

Do not build an unvalidated tunnel or maglev physics solver inside the decision layer. Consume qualified engineering envelopes or specialist solvers through explicit adapters and preserve their context of use.

## 6. Underground reuse gate

Existing tunnels, mines, shafts, galleries and brownfield assets are candidates only after identity/ownership, rights, as-built survey, dimensional uncertainty, rock mass/structural condition, groundwater, connected workings, contamination, gas, ventilation, fire/egress and access evidence is reviewed for the exact proposed duty.

Compose [BROWNFIELD-UNDERGROUND-001 #1951](https://github.com/Luminous-Dynamics/symtropy/issues/1951). An existing void is not free capacity and is not safe just because its geometry appears suitable. Reuse, remediation/closure, new construction and no-build remain competing portfolio alternatives.

## 7. Whole-life objective vector

Keep financial, fiscal and social ledgers separate. Report at least:

- generalized door-to-door freight cost per accepted tonne/TEU, with access, wait, transfer, delay variability and inventory effects;
- end-to-end time distribution and service-level attainment, not only top speed;
- delivered throughput at declared demand and terminal/route bottleneck utilisation;
- lifecycle capex, finance, energy, staffing, operations, maintenance, renewal, replacement, insurance, decommissioning and residual value;
- energy per tonne-km and peak MW by route/vehicle/terminal, with grid mix and fuel factors source-pinned;
- lifecycle greenhouse gas, local air pollution, noise, land take, water use, ecological disturbance and construction spoil;
- construction cost/schedule distribution and utilisation break-even demand;
- service loss, unmet demand, recovery time and inventory exposure under deterministic N-1 and common-cause fault sets;
- distribution by region/community/worker/user/taxpayer and affordability/access effects;
- regret versus exact finite-case oracle, Pareto hypervolume gap, optimality gap, compute and evaluation budget.

No weighted objective may override required fire/egress, structural/geotechnical, operational-control, environmental, rights or minimum-service constraints.

## 8. Frozen benchmark corpus

V0 should contain at least 24 hand-checkable synthetic corridor cases, stratified by:

- port-yard transfer, 50–100 km regional corridor, 300–700 km intercity corridor and longer trunk corridor;
- flat, mountainous, dense urban, flood-prone, water-scarce and seismic geology profiles;
- functioning rail, constrained rail, degraded rail and no usable rail;
- high, medium and low demand with growth, stagnation and demand-collapse variants;
- container-only, bulk-only, automotive/general freight and mixed cargo;
- conventional rail, partial tunnel, full tunnel, MagRail retrofit, dedicated maglev, pod, conveyor/pipeline and no-build;
- one or multiple terminals, variable dwell and constrained transfers;
- construction delay/cost escalation, low utilisation and financing-rate shock;
- single-point power, control, portal, ventilation, bridge, border and terminal failures;
- independent access/power options and recovery/repair capacity;
- legal/rights/environmental uncertainty as hard-gate negative controls.

Each fixture includes exact input manifest, explicit parameter provenance class (synthetic in V0), expected feasibility disposition, simple reference route/flow result and fault-domain graph. Preserve zero-demand and no-build controls.

## 9. Baselines and acceptance criteria

Compare to:
- exact exhaustive enumeration on small finite cases;
- Dijkstra/A* for deterministic single-commodity shortest path;
- standard min-cost flow for capacitated flow assignment;
- a transparent MILP or CP-SAT baseline for discrete design and schedule choices;
- a Pareto evolutionary baseline (e.g. NSGA-II) where the objective vector warrants it;
- greedy/nearest-port and mode-specific simplified baselines to expose shortcut errors.

Acceptance targets (proposed, not achieved):
- 100% fail-closed disposition for frozen unknown-rights, unsafe-egress, geotechnical, insufficient-capacity and invalid-unit negative controls;
- exact agreement with independent path/flow oracles on all small cases;
- disclose optimality gap and solver/runtime settings for each baseline comparison;
- no candidate is called feasible on synthetic throughput or assumed reliability alone;
- on larger budgets, test at least 30% fewer expensive candidate evaluations than exhaustive enumeration while retaining all reference-feasible alternatives; publish precision/recall and regret so the target cannot hide false positives;
- use held-out corridor topologies and parameter regimes; no adjacent tile leakage or tuning on the test set;
- report all trade-offs as Pareto vectors, not one marketing scalar.

## 10. South African baseline and first study

Transnet describes ContainerCor as a 688 km Durban–Gauteng railway linking the port to City Deep, Kascon, Pretcon, Kaalfontein and private sidings. It is the real network baseline for any South African port-to-inland comparison, not proof of current spare capacity. Use the latest Network Statement/Network Register, point-to-point running-time and capacity statements to qualify actual section capacity, restrictions, allocations and timetable applicability before modelling a service. ContainerCor source: https://www.transnet.net/SubsiteRender.aspx?id=4492347

Recommended first synthetic-to-data progression:
1. Freeze a small synthetic corridor family and independent flow oracle.
2. Create a ContainerCor source manifest and qualify section-level capacity/time inputs via the transport data owner (#1631).
3. Use a matched OD and commodity profile across rail-recovery, new rail, MagRail retrofit, dedicated tunnel/maglev and no-build alternatives.
4. Compare the full lifecycle and service outcomes. Do not start with a preselected technology.
5. Only after evidence and operator/stakeholder access, investigate a port-yard bottleneck or specific constrained link as a pilot hypothesis.

## 11. Claim ceiling

This benchmark chooses which mode/route deserves further investigation. It does not certify tunnel safety, geotechnical design, rail compatibility, electromagnetic compatibility, maglev maturity, fire/life safety, rights, environment, procurement or commercial viability. Synthetic performance is a software contract, not measured service performance.