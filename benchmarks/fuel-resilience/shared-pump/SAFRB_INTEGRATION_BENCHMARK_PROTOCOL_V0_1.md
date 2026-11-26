# SAFRB integrated assurance benchmark protocol v0.1

**Status:** proposed benchmark design; experiments not yet run  
**Parent:** SAFRB deliverability #7483; global interoperability profile #7507  
**Known-answer:** `SAFRB-SHARED-PUMP-2ROUTE-001`  
**Scenario SHA-256:** `0a1c507d3bff9b3062e24ca8718e102244473d839c2a31c5ead31812ee249be2`

## 1. Research question

Does composing physical feasibility, federated claim/evidence qualification, decision-aware evidence acquisition and read-only spatial projection produce **more defensible essential-service decisions** than the strongest matched single-function or human baselines, under the same evidence, time, cost, permission and safety budgets?

This is an end-to-end composition benchmark, not a replacement for each repository's owner-native component suite.

## 2. External systems and research baselines

The benchmark must compare against existing capabilities honestly. The named products are functional reference points; this protocol does not claim to have run them or reproduce their internal algorithms.

| System / programme | Publicly documented capability | SAFRB relationship and fair comparison |
|---|---|---|
| DTN TIMS | Automated terminal inventory gathering, real-time inventory and receipt/loading events, product/location/time views and alerts | Ingest its admitted inventory/event outputs as candidate evidence. SAFRB tests whether stock is admitted, accessible, transferable and sufficient for a scoped service obligation. Do not claim TIMS lacks inventory visibility. |
| AVEVA PI System | Collects, stores, normalizes, contextualizes, visualizes and delivers operational data across industrial assets | Treat as a source/historian/context layer. Test lineage, freshness, source dependence and measurement admission at the assurance boundary; do not replace historian functionality. |
| Schneider Electric EcoStruxure Microgrid Advisor | DER monitoring, forecasts, APIs and model-predictive microgrid optimization | Consume its declared forecasts/plans as candidate inputs. Independently qualify external fuel/resource paths and beneficiary-bound service evidence. Do not rank against a different EMS objective or boundary. |
| Siemens Gridscale X | Integrates grid planning, operations, asset management and metering through a shared digital-twin platform | Treat as a grid operations/digital-twin comparator. Test cross-owner, cross-sector service claim semantics and independent evidence preservation, not generic dashboard breadth. |
| NIST SI for Manufacturing / TIDAL | Research architecture for trustworthy industrial intelligence, human-machine teaming, contextualized data, digital-twin VVUQ, integration test bench and benchmark studies | Align with its measurement science, context ablations, human teaming and test-bench discipline. SAFRB's narrower hypothesis is infrastructure-owner federation and proof-qualified resource-to-beneficiary services. |
| NIST IR 8536 | Manufacturing traceability meta-framework using linked provenance, cryptographic links and selective disclosure without a centralized repository | Reuse these traceability and selective-disclosure principles through Mycelix. SAFRB adds authority recognition, physical deliverability and service qualification; do not reinvent a generic provenance chain. |

Sources:
- DTN TIMS: https://www.dtn.com/refined-fuels/terminal-operator/tims/
- AVEVA PI System: https://www.aveva.com/en/products/aveva-pi-system/
- Schneider Electric EcoStruxure Microgrid Advisor: https://www.se.com/us/en/product-range/65896-ecostruxure-microgrid-advisor/
- Siemens Gridscale X: https://www.siemens.com/en-us/products/gridscale-x/
- NIST SI for Manufacturing / TIDAL: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
- NIST IR 8536: https://csrc.nist.gov/pubs/ir/8536/final 
### 2.1 NIST TIDAL: methodology comparator, not a claimed invention

NIST's current SI for Manufacturing programme describes a shared operation test bench spanning discrete, additive and continuous processes, using real equipment and simulators. Its planned benchmark tasks include production planning, scheduling, process monitoring and root-cause analysis. Its integration thread is comparing information architectures including unified namespace over MQTT with knowledge graphs/type models and I3X/OPC UA profiles. The programme names accuracy, latency, cost and grounding as benchmark dimensions, and its fault-diagnosis study prioritizes detection recall ahead of precision. SAFRB should adopt the measurement discipline and context-ablation approach, but must not imply that a SAFRB fixture has been tested on this bench or that its cross-owner service-assurance question replaces NIST's manufacturing scope.

For SAFRB, add an explicit *grounding* audit per decision: every claimed resource, capacity, time window, authority and service boundary must resolve to the exact admissible source artifact and profile version that was available at decision time. Report grounding as its own metric; do not merge it into general accuracy. For fault detection/diagnosis arms, report recall and precision separately, and preregister the consequence-weighting for missed faults. Do not import NIST's fault-recall weighting as a universal ranking rule for allocation, investment or service scheduling.



### 2.2 Graph-kernel matched solver baselines

The graph algorithm itself is not a novel contribution claim. The v0 kernel uses deterministic integer-capacity Dinic. It must be benchmarked against mature, independently implemented solvers on the *same canonical graph instances*:

| Baseline | Algorithms | Why include it | Comparability constraint |
|---|---|---|---|
| Google OR-Tools `SimpleMaxFlow` | OR-Tools' max-flow implementation | Independent established solver; returns per-arc flow and exposes source/sink minimum-cut partitions | Pin exact release/build; use the same integer capacities and explicit node/arc mapping |
| NetworkX 3.7 `preflow_push` | Highest-label preflow-push | Different algorithmic family and current documented default for `maximum_flow` | Pin exact NetworkX release and explicitly name `flow_func`; do not rely on a changing default |
| NetworkX 3.7 `dinitz` | Dinitz blocking-flow implementation | Independent implementation of the same broad algorithm family | Pin release; compare value, per-arc flow feasibility and cut equality, not identical witness allocation |
| NetworkX 3.7 `edmonds_karp` | Edmonds-Karp | Slower but useful as a transparent small-graph cross-check | Use on the exhaustive/small corpus, not as the scale performance target |

Official references: OR-Tools maximum-flow API, including per-arc flow and min-cut access: https://developers.google.com/optimization/flow/maxflow. NetworkX flow algorithms and the `maximum_flow` default/selection semantics: https://networkx.org/documentation/stable/reference/algorithms/flow.html and https://networkx.org/documentation/stable/reference/algorithms/generated/networkx.algorithms.flow.maximum_flow.html.

### 2.2.1 DIMACS Network Flows 2.0 compatibility track (2026–2027)

The current 13th DIMACS Implementation Challenge is explicitly refreshing flow benchmarks because many historic instances have become too easy for modern implementations. Its goals include updated benchmark instances from real applications, new algorithm implementations and comparison of older/newer methods. Its max-flow reference page names HIPR4 as the principal baseline and also lists pseudoflow and EIBFS; it says no reference implementation universally dominates and recommends comparing against all relevant codes. SAFRB should adopt this as its external algorithm-performance track rather than treating a small NetworkX sample as SOTA evidence.

**Required reference set and corpus mapping:**

| External baseline / source | SAFRB use | Important scope boundary |
|---|---|---|
| HIPR4 v4.2 (highest-level push-relabel) | Principal mature max-flow reference on the DIMACS-compatible static corpus | Match DIMACS input and exact capacity semantics; isolate parser/translation cost from solver time |
| Pseudoflow (C implementation) | Alternative algorithm family and practical performance comparison | Record DNF/timeouts; do not cherry-pick only easy cases |
| EIBFS / incremental breadth-first-search enhancement of Boykov–Kolmogorov | Image/vision graph cases and incremental graph variants where its model applies | Its vision specialization means results must be stratified by graph family; do not generalize its performance to fuel/hydraulic topologies |
| DIMACS max-flow core instances | Vision, segmentation, surface-fitting, stereo, punch, bisection and road-network partition families | Freeze dataset version, source URL, conversion tool and canonical digest for every instance |
| DIMACS generators | RMF long/wide and Washington RLG long/wide | Pin exact generator source, parameters, seed and patched revision; publish the generated graph itself and its digest |

The challenge's reference pages specify core instance families, including vision, PUNCH and bisection, and list Polygon Aggregation instances with 64-bit capacities even though the general format describes 32-bit capacities. This makes a 64-bit parser/runner lane particularly relevant to the kernel's u64 contract. Keep the ordinary DIMACS compatibility lane within the format's defined numeric range; run u64 boundary/Polygon cases in a separate, explicitly labelled extension lane. Some challenge datasets include incremental instances. Preserve those as a separate dynamic-update lane, not mixed into static one-shot solver timings.

**Submission-style reporting:** publish all required core instance results; report DNF when a solver does not finish; include additional domain-relevant instances; give instance provenance and digests; report hardware, compiler, library and solver versions; provide per-case timing and memory, not just one aggregate score. For the SAFRB adapter, add the end-to-end graph construction, certificate verification and evidence-link resolution time alongside the bare solver result.

**SOTA rule:** a faster max-flow value alone is not a new field-level result. The kernel must match the mature reference solvers on every supported instance while preserving exact edge identities and independently verifiable primal/dual certificates. SAFRB's differentiator is the evidence-bound service assurance layer—joint/individual semantics, physical-resource alias handling, evidence freshness and portable verification—not an unsupported claim to have invented a faster max-flow algorithm.

Official challenge sources:
- [Network Flows 2.0 challenge overview](https://coral.ise.lehigh.edu/flow-challenge-2-0/) — challenge motivation, goals and problem classes.
- [Max-flow instances and generators](https://coral.ise.lehigh.edu/flow-challenge-2-0/max-flow-instances-and-generators/) — core real-world families, generators and 64-bit capacity note.
- [Max-flow reference solvers](https://coral.ise.lehigh.edu/flow-challenge-2-0/max-flow-reference-solvers/) — HIPR4, pseudoflow and EIBFS reference programs and comparison guidance.

**Graph equivalence is a benchmark precondition.** NetworkX documents that `MultiDiGraph` inputs are not supported by its maximum-flow functions. For parallel edges, either use a separately tested capacity-preserving node-splitting transformation with explicit reverse mapping, or exclude those cases from that comparator; never silently coalesce arc identities and then compare an identity-sensitive certificate. OR-Tools and the in-house kernel should retain distinct arc identities. Restrict cross-solver comparisons to the common exact integer range; exercise u64 overflow behavior separately in the Rust kernel's own tests.

**Corpus.** Include (a) all small directed topologies through four nodes, (b) sparse and dense seeded graphs, (c) layered/time-expanded DAGs, (d) shared physical-resource capacity gates, (e) parallel and antiparallel arcs, (f) zero-capacity and disconnected cases, (g) high-capacity/skewed distributions, (h) long paths, and (i) adversarial alias/bypass mutations. Publish the seed, canonical graph digest, graph size, degree distribution, capacity distribution and model class for every case.

**Measurements.** Report solver value, feasible edge-flow witness, independently reconstructed cut capacity, independent certificate verification result, input normalization time, solver time, witness construction time, verifier time, peak memory and p50/p95/p99 over repeated runs. Compare end-to-end adapter+solver+verification separately from the bare algorithm. Report graph sizes and hardware/software versions; no aggregate score may hide a correctness failure.

**Decision rule.** The acceptance target is exact agreement on flow value and cut value over every supported graph, 100% certificate acceptance for valid outputs, and 100% rejection for mutated certificates. Performance goals are workload-specific p95 budgets, not a blanket claim to outrun OR-Tools or NetworkX. If another mature solver is faster for a supported case, use it as the engine behind the same independently verified certificate contract. The intended differentiation is the evidence-bound resource-to-beneficiary service assurance layer, not a new claim to have invented or universally improved max-flow.

### 2.3 Different optimization classes need different baselines

Do not extend the max-flow comparison to models it cannot represent.

| Model class | Matched baseline | Use it for | Qualification boundary |
|---|---|---|---|
| Single-commodity, cost-free flow | OR-Tools `SimpleMaxFlow`; NetworkX preflow-push / Dinitz / Edmonds-Karp | Maximum feasible quantity and minimum-cut comparison | Same directed graph, integer capacities, source/sink and edge-identity mapping |
| Single-commodity, declared linear unit costs | OR-Tools `SimpleMinCostFlow` | Cost-minimizing flow under arc capacities and node supplies/demands | Costs must be owner-supplied, dimensioned and versioned. An objective coefficient or dual value is not a market price, asset value or authorized intervention cost |
| Linear process-coupled constraints | HiGHS LP | Shared linear constraints, conversion balances and process-coupled resource allocation | Compare primal feasibility, objective and independently checked dual/Farkas evidence where available; retain solver/version/tolerances |
| Integer schedules and discrete decisions | HiGHS MILP | Unit commitment, batch sizes, vehicle/crew slots and discrete restoration choices | A solver-reported optimal/infeasible status is not by itself a portable proof. Preserve exact model/input digests, bounds, incumbent, optimality gap and an independently checkable proof/replay route; otherwise report solver-qualified, not proof-certified |
| Nonlinear / nonconvex physics | Domain-specific simulator/optimizer with VVUQ | Pump curves, hydraulic pressure, thermal limits, nonlinear conversion and safety envelopes | Keep the physical-model class separate; require uncertainty, calibration, applicability and independent model validation |

OR-Tools documents `SimpleMinCostFlow` as a capacitated flow model with node supplies/demands and integer unit costs; HiGHS documents LP, MILP and QP support (but not mixed-integer QP). Sources: https://developers.google.com/optimization/flow/mincostflow and https://ergo-code.github.io/HiGHS/stable/.

The DIMACS Implementation Challenges are a useful *benchmark-methodology* comparator: they emphasize realistic instance generators, empirical implementation behavior and practical algorithm performance rather than relying on worst-case bounds or unrealistic random graphs alone. Apply that discipline to SAFRB's sparse, layered, time-expanded and common-cause topologies; this is a methodology reference, not a claim that SAFRB already participates in a DIMACS challenge. Source: https://dimacs.rutgers.edu/programs/implementation-challenges/.

## 3. Matched experimental arms

Every arm receives the same frozen decision-time observation, service requirement, resource graph, candidate actions, cost/time budgets and held-out case IDs. The product rows above are not silently simulated as if they were vendor runs.

| Arm | System under test | What it isolates |
|---|---|---|
| A | Nominal dashboard / inventory-only decision | Common failure mode: nominal stock and declared capacity treated as available service |
| B | Fixed diagnostic checklist | Non-adaptive human-authored evidence sequence |
| C | Maximum / greedy information gain | Information reduction without service-decision utility |
| D | Decision-theoretic EVSI / VoI | Decision value with matched costs, delays and supported model family |
| E | Symtropy physical certificate only | Physical feasibility under admitted capacity inputs |
| F | Symtropy + Mycelix | Added identity, authority, freshness, provenance and common-cause qualification |
| G | F + Symthaea | Added proof-obligation-directed, model-robust investigation policy |
| H | G + Sol-Atlas projection | Added spatial/temporal observability; no change to canonical qualification |
| I | Human/operator investigation | Expert baseline under the same information, permission, time and cost budget |

Evaluate the full stack as H plus the Power Fabric service-claim adapter. The adapter must preserve claim scope and disposition, not recalculate the physical model.

## 4. Required scenario families

1. **Single-commodity physical flow:** shared pump, shared berth, shared pipeline segment, shared power feeder, capacity lower/upper bounds, outages and time-window cut sets. Use max-flow/min-cut only for network forms that satisfy its assumptions.
2. **Process-coupled and multi-commodity:** fuel grades, product compatibility, nonlinear conversion, storage, integer schedules and multi-resource constraints. Use an appropriate independently checkable LP/MILP/constraint proof; otherwise return unsupported, unproven or indeterminate.
3. **Evidence ambiguity:** stale, missing, revoked, partial, conflicting, future, replayed and out-of-scope observations; invalid calibration; correlated dashboards; alias substitution; source common causes.
4. **Authority and federation:** key authentication versus purpose recognition, title/custody/contract/access versus operational permission, revoked and conflicting credentials, selective disclosure, offline reconciliation and cross-jurisdiction profile changes.
5. **Active investigation:** actions with different ability to resolve a proof obligation, cost, delay, downtime, safety prerequisite, source dependence, refusal and inconclusive outcomes.
6. **Geospatial and temporal projection:** coordinate uncertainty, different CRS/datum, non-whole-hour offsets, daylight-saving folds/gaps, stale positions, offline updates, duplicated assets and cross-layer confusion.
7. **Beneficiary service and equity:** critical service floors, priority classes, rural/remote cohorts, affordability, recovery time and unserved demand. Keep service minima non-compensatory.
8. **Counterfactual service restoration:** pump-capacity uplift, genuinely independent alternate resource, source/stock replenishment, delivery rescheduling where the beneficiary contract permits it, demand-side alternatives and restoration time. Each candidate intervention must state its required evidence, authority, lead time, cost basis, failure-domain dependencies and which per-window proof obligations it changes. A mathematical threshold is not an asset rating, price or authorization.

## 5. Non-compensatory gates

A run fails qualification if any of these occur:

- false guaranteed service on any hard-negative control;
- unsupported model class or invalid certificate accepted as proof;
- shared physical resource counted more than once;
- stale, future, revoked, conflicting or out-of-scope evidence promoted into a current guarantee;
- issuer key authentication treated as purpose recognition, title or dispatch authority;
- authority, certainty, currentness, specificity or evidence maturity strengthened by an adapter;
- unauthorized disclosure, inspection request, OT write, inventory mutation or equipment actuation;
- communication reachability treated as physical delivery, or a spatial visualization treated as physical feasibility;
- unknown dependency represented as independent;
- failure of an explicitly required beneficiary service floor.

A run that rejects everything is not useful: supported positive controls and unsupported/ambiguous controls must both be present and measured.

## 6. Metric vector (do not collapse to a weighted score)

**Physical:** exact flow/witness/cut agreement, conservation, false guarantees, false rejection, shared-resource double-counts, unsupported model refusals, capacity-bound sensitivity, p50/p95 verifier latency. Independently verify that the declared source-side set is exactly residual-reachable from the submitted joint or isolated edge-flow witness; reject a substituted equal-capacity cut partition. Distinguish (a) joint feasibility across all service floors, (b) each beneficiary's independently maximized deliverable quantity, and (c) the allocation in one joint feasible witness. Individual maxima are not additive when resources are shared; never label one witness allocation a beneficiary-level guarantee. Benchmark the accuracy of all three outputs separately, including adversarial cases where independent maxima exceed the joint optimum.

**Evidence and federation:** source-to-output traceability, semantic preservation, stale/conflict/revocation detection, alias/common-cause fidelity, authority/certainty/currentness/specificity promotion count, disclosure excess, offline conflict preservation.

**Investigation:** decision regret, false-safe rate, justified-decision change rate, proof-obligation coverage, redundant tests, acquisition cost, elapsed time, downtime, refusal and human-escalation rates, model-adequacy failures.

**Spatial:** topology and identity fidelity, route/resource alias correctness, CRS/time transformation errors, stale-position promotion, source maturity and claim-state preservation.

**Beneficiary outcomes:** critical service-hours delivered, worst-cohort shortfall, unserved demand, affordability, recovery time, environmental impact and local operator participation.

**Counterfactual restoration:** minimum additional *qualified lower-bound* capacity by resource and time window; alternative independent-resource requirement; earliest evidence/authorization deadline; time-to-restore; intervention Pareto frontier across capital/operating cost, downtime, emissions, service reliability, privacy and operator burden. Report unsupported options and evidence debt explicitly. Do not interpret max-flow cut values or LP dual variables as market prices.

**System overhead:** end-to-end assurance latency as a fraction of the actual decision budget; compute, data transfer, human review and integration cost.

## 7. Prospective targets (not achieved results)

| Gate / target | Initial research target | Qualification note |
|---|---:|---|
| False guaranteed service | 0 / 3,000 stratified hard-negative cases | Report exact denominators and one-sided confidence interval; cases must span independent topology/failure families, not merely edge permutations |
| Positive-control false rejection | <=1% on >=300 supported positive cases | Unsupported cases are reported separately, not counted as positives |
| Flow/cut and supported certificate verification | 100% on all known-answer panels | Independent verifier implementation and frozen inputs |
| Semantic or authority promotion | 0 | Across all adapters, profile upgrades and locale transformations |
| Undeclared disclosure / unauthorized operational effect | 0 | Adversarial privacy and permission tests |
| Decision regret | >=20% lower than the strongest matched checklist / EIG / EVSI baseline | No increase in false-safe rate; include human/operator arm |
| Assurance overhead | P95 <=10% of available decision time budget | Include evidence acquisition delay separately from compute latency |
| Conformance breadth | 10 jurisdiction overlays, 5 languages/scripts, 4 connectivity modes | Synthetic conformance first; no claim of actual jurisdictional approval |
| Outcome distribution | No breach of declared critical floor in any protected cohort | No average score may compensate for a cohort floor breach |
| Counterfactual frontier integrity | 100% of emitted options bind to exact per-window proof obligations, input digests and owner/authority prerequisites | Mathematical threshold must remain separate from market price, asset capability and operational recommendation |

For rare-event gates, publish exact binomial bounds and dependence assumptions. Zero errors in 3,000 independent negative cases supports an approximate one-sided 95% upper bound near 0.1%; correlated test variants do not count as independent trials.

## 8. Experimental protocol

- Freeze input cases, model-family definitions, source identities, resource dependency graphs, legal/profile assumptions, allowed actions, budget and artifact digests before a run.
- Separate planner-visible observations from oracle-only truth. No future receipt, assay, outage, title outcome or service result may leak into the planner.
- Use topology-family and jurisdiction-family holdouts, not random row splits. Include unseen combinations of route, common cause, evidence provider, locale and connectivity.
- Match compute budgets and information access. Report compute used, operator time, acquisition delay and refusal.
- Run ablations by removing exactly one component at a time. Attribute improvements only to the component whose contribution survives the ablation.
- Require independent implementations for certificate verification and the small-instance optimal-policy oracle. Do not share the candidate algorithm's core helper with its verifier.
- Publish all failures, abstentions, counterexamples, denominators, confidence intervals, commit SHAs and source digests.
- For intervention frontiers, compute small-instance minima by exhaustive enumeration; for larger models, use a solver only when its model class and proof/optimality certificate are independently verifiable. Report feasible incumbents, optimality gaps, unproven minima and unsupported options separately.
- Keep synthetic, replay, SIL, HIL, commissioning and field evidence as separate claim classes.

## 9. SOTA claim rule

A superiority claim is permitted only when the full integrated system beats the strongest matched open, commercial-accessible or human baseline on a preregistered held-out set, under identical data, budgets and service boundaries, with no hard-gate regression. Vendor feature descriptions and synthetic known-answer passes are not benchmark results.

A credible first win is not "better forecasting" or a higher aggregate score. It is **fewer false service guarantees, lower decision regret and lower evidence-acquisition burden while preserving authority, provenance, privacy and critical-cohort service floors**.

## 10. Deployment/test ladder

1. Synthetic known-answer and adversarial fixtures.
2. Frozen historical replay with independently checked source and event lineage.
3. Shadow integration against read-only vendor exports and owner-authorized records.
4. Lab/HIL for applicable microgrid control/energy-management systems, aligned to IEC TS 62898-3-5:2026 where in scope.
5. Commissioning and periodic requalification under local authority and operator-approved test plans.
6. Field evaluation with beneficiary service measurement and independent audit.

No step is implied by passing the previous step. This protocol does not authorize OT control or physical tests.
