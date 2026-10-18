# INDUSTRIAL-COUPLING-001: Coupled production, utilities, and inter-plant resource optimization

**Status:** Research/benchmark proposal v0.1  
**Date:** 2026-10-18  
**Scope:** Offline, deterministic benchmark contract. No plant control, field-performance claim, or claim that a new optimizer already beats established methods.

## Executive question

Can joint optimization of production schedules, process heat, inventories, shared equipment, and inter-plant exchanges deliver better *verified saleable output and service* than independently optimizing those subsystems—without violating physical constraints, worsening tail risk, or making one participant absorb another's costs?

The central hypothesis is **non-additive system value**: optimizing each subsystem separately can miss opportunities or create conflicts because time, temperature grade, equipment, material quality, transport, storage, and participant incentives couple their feasible sets. The benchmark must also permit the null result: coordination overhead, distance, incompatible temperatures, uncertainty, or poor contract terms may make joint operation worse.

## Why this is the next seam

Related work already exists and should be composed rather than reimplemented:

- [Symtropy #1967](https://github.com/Luminous-Dynamics/symtropy/pull/1967): deterministic validation of industrial workcell schedules; it explicitly does not perform schedule optimization.
- [Symtropy THERMAL-PINCH-001 #1899](https://github.com/Luminous-Dynamics/symtropy/issues/1899) and [THERMAL-DYNAMIC-001 #1916](https://github.com/Luminous-Dynamics/symtropy/issues/1916): heat-grade matching, pinch targets, storage state, and multi-period uncertainty.
- [Symtropy THERMAL-OPT-001 #1877](https://github.com/Luminous-Dynamics/symtropy/issues/1877): matched-budget dispatch comparisons and held-out Pareto evaluation.
- [Symtropy MINE-SYSTEM-001 #1933](https://github.com/Luminous-Dynamics/symtropy/issues/1933): a domain-specific campaign benchmark that optimizes accepted saleable output rather than machine activity.
- [Sol-Atlas #160](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/160): evidence-qualified industrial capacity; [#159](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/159): spatial material-flow context; [#116](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/116): evidence-aware spatial-economic resilience.
- [Mycelix MFG-001 #4685](https://github.com/Luminous-Dynamics/mycelix/issues/4685): manufacturing execution evidence and qualification states; [MFG-PROC-001 #3085](https://github.com/Luminous-Dynamics/mycelix/issues/3085): process registry/routing seam.

The gap is an **integration benchmark** for the value, cost, and failure modes of coupling these otherwise separate capabilities. This proposal does not replace their local models or assert that all should be put in one runtime.

## Paradigm shift

From: maximize one machine's utilization, minimize one plant's utility bill, or improve one facility's nominal throughput.

To: maximize the value of *accepted output and required service delivered by the whole production network*, subject to hard physical, quality, safety, schedule, environmental, and contractual constraints.

Three rules make this scientifically meaningful:

1. **Conservation before optimization.** A better objective score is inadmissible if mass, energy, inventory, time, or exclusive capacity is double-counted.
2. **Hard floors before weighted scores.** Safety, product acceptance, environmental limits, and protected service floors are non-compensatory gates, not penalties that profit can buy away.
3. **Compare regimes, not just algorithms.** Measure what local optimization, cross-subsystem coordination, and inter-plant exchange add—and where they do not help.

## Smallest useful reference case

Build a synthetic, versioned two-facility network with a deliberately small exact-oracle instance and a larger stress-test family.

- **Facility A:** batch production jobs with precedence, setup/changeover time, machine exclusivity, yield, and quality acceptance.
- **Facility B:** another production process with its own jobs, quality requirements, capacity, and deadlines.
- **Thermal subsystem:** hot streams and heat sinks with explicit temperature grade, duties, time windows, minimum approach, storage state, losses, and backup utilities. Start with the heat-stream subset that current kernels support; do not silently assume a synthesized exchanger network.
- **Material subsystem:** lot quantities, grades, inventory balances, lead time, recovery yield, and destination acceptance. A declared by-product is not automatically a usable input.
- **Shared link/asset:** an optional heat or material exchange whose capacity, distance, transfer loss, operating window, maintenance, cost, and availability are explicit.
- **External conditions:** electricity/fuel price traces, utility caps, emissions factors, demand, forecast error, and failure/outage events.
- **Economic participants:** at least one decision-making owner per facility, with declared cost responsibility and benefit-allocation terms. Do not assume the network's total surplus is distributed fairly by itself.

Every fixture must declare units, time basis, scenario lineage, evidence class, model version, uncertainty, and digest. Synthetic values must remain visibly synthetic and must never be reported as observed industrial performance.

## Executable v0.1 slice now present

The branch also contains a first offline Rust reference package at [`crates/tools/industrial-coupling-benchmark/`](../tree/research/industrial-coupling-benchmark-v0-1-2026-10-18/crates/tools/industrial-coupling-benchmark). It implements exact enumeration over a bounded integer-kWh heat-transfer domain, a constant-temperature grade screen, heat-loss and cost ledgers, two-party surplus allocation subject to each facility's minimum participation benefit, and a separate finite-domain optimality scan.

It includes one synthetic positive case plus negative controls for incompatible temperature grade, uneconomic transfer, zero capacity, tampered accounting, invalid allocation shares, reservation floors that veto otherwise-positive cooperation, and an excessive search bound. Its ten unit tests are authored, but were **not executed in this editing environment** because a Rust toolchain was unavailable. GitHub Actions were queued at the latest exact-head inspection; no test pass is claimed.

This is intentionally narrower than INDUSTRIAL-COUPLING-001: it does not model time-dependent storage, temperature profiles, a heat-exchanger network, production schedules, real tariffs, measured facilities, or actual savings. It validates one integration seam and makes the next expansion testable without pretending the whole platform has been implemented.

## Executable robust-decision slice now present

The standalone Rust crate now includes `evaluate_robust` and `verify_robust_plan` for finite-scenario evaluation. One common integer-kWh transfer must pass physical checks and both participants' reservation floors in every listed scenario. The evaluator maximizes worst-case surplus, breaks positive worst-case ties by total surplus and then lower transfer volume, and keeps zero exchange when no robust benefit exists. Scenario IDs must be unique; the evaluator enforces a 64-scenario and 2,000,000 scenario-evaluation cap.

A deterministic two-scenario fixture is independently enumerated in a separate calculation: the selected transfer is 42 kWh input, with 37 kWh and 35 kWh delivered in the nominal and conservative cases respectively, and worst-case net surplus of 44,200 synthetic milli-credits. This arithmetic was checked independently; it is not a Rust test result or a measured plant result.

The first robust slice is intentionally conservative: it chooses a common fixed decision, does not assign probabilities, and does not model recourse. Follow-on research should compare common-decision robust optimization against adaptive policies, calibrated stochastic models, regret/CVaR objectives, and strong conventional optimizers under matched information and evaluation budgets. The decision criterion should be promoted only if held-out cases show it makes better trade-offs between economics and operational risk.

## Tournament: mandatory comparison arms

All arms receive the same plant model, constraints, demand/price/failure information, and declared compute budget.

1. **Rule baseline:** explicit fixed-priority production and utility policies.
2. **Independent optimization:** each facility optimizes its schedule and utilities separately; cross-plant exchanges are disabled or decided by a fixed rule.
3. **Sequential coordination:** optimize a subsystem, pass its outputs to the next, then re-evaluate feasibility. This measures ordering effects.
4. **Joint exact oracle:** exhaustive enumeration or a suitable exact mixed-integer/constraint solver for tiny cases, with a recorded optimality certificate/gap where the solver provides one.
5. **Joint scalable baseline:** a strong conventional MILP/CP-SAT, decomposition, or rolling-horizon method on larger cases, selected per problem structure.
6. **Robust / scenario-aware method:** evaluate uncertainty explicitly using a defined finite scenario set, robust constraints, or a declared distribution—not an unqualified point forecast.
7. **Symthaea-guided search (optional treatment):** candidate generation, prioritization, or replanning may be evaluated as an additional method. It must not receive extra privileged information or simulator calls. Remove it in an ablation, and retain it only if it provides repeatable value over the strongest conventional baseline.

An optimizer's result is always passed through an independent feasibility verifier. The optimizer cannot certify its own answer merely by reporting success.

## Metrics and accounting

Report the following independently; do not compress everything into one opaque scalar.

### Hard correctness gates

- All accepted schedules pass precedence, time-window, resource exclusivity/capacity, and changeover checks.
- Every material lot and energy/heat flow reconciles across inputs, conversion losses, inventory changes, outputs, and disposal within declared numerical tolerances.
- No mass, thermal duty, storage state, inventory, or shared capacity is allocated to two consumers at the same time.
- Temperature-grade, transport, quality, time, availability, unit conversion, and contract restrictions are enforced when applicable.
- A failed or unknown check is **unqualified**, not silently treated as pass.

### Operational and environmental outcomes

- Accepted, specification-grade output; on-time delivery; yield/scrap/rework; cycle time and bottleneck utilization.
- Fully loaded cost per accepted output, including energy, consumables, setup, maintenance, storage, transport, disposal, and explicitly modeled transaction/integration cost. Report CAPEX separately and state the amortization horizon.
- Useful heat delivered by temperature grade, exergy where modelled, purchased electricity/fuel, storage losses, water use, and emissions using explicit system boundaries.
- Unmet critical service, production lost during disruptions, recovery time, backlog, and tail outcomes—not only mean performance.
- Runtime, memory, solver evaluations, optimality gap, and integration/data preparation burden.

### Cross-participant economics

- Total network surplus and each participant's net outcome against its own feasible no-cooperation baseline.
- Shared-asset utilization and costs, sensitivity to energy/material prices, and who bears downside risk.
- An explicit allocation mechanism or contract; compare at least one simple proportional allocation with one negotiated/cooperative alternative where appropriate. Individual-rationality is a measured property, not an assumption.
- Report concentration, dependence on a single supplier/route, and any distributional downside rather than treating aggregate surplus as sufficient.

## Required ablations and negative controls

1. **No beneficial coupling:** a fixture where the inter-plant link has zero useful capacity or exchange cost exceeds its realizable value. The optimizer must not invent savings.
2. **Incompatible grade:** heat is available but below the sink's required grade and no valid heat pump exists. Direct heat transfer must be rejected.
3. **Unavailable resource:** an outage closes a machine, link, or storage unit; stale availability must not be used as current capacity.
4. **Material decoy:** a by-product fails a quality/contamination specification. It cannot be credited as recovered feedstock.
5. **Double-counted stream:** deliberately duplicate a lot or heat source. The independent verifier must reject the plan.
6. **Missing or stale evidence:** required capacity, tariff, calibration, or provenance is absent or expired. The affected claim becomes unqualified; imputation must be labeled and sensitivity-tested.
7. **Adverse price/forecast shift:** utility or feedstock economics reverse. Report regret and the transition policy, not just the training scenario's score.
8. **Partner withdrawal / contract change:** the exchange becomes unavailable or the allocation agreement changes; the result must remain feasible or clearly fail closed.
9. **No-coupling equivalence:** when cross-system constraints are deliberately removed, joint optimization should match or beat the independent baseline only within solver/tolerance limits; any claimed large coupling premium is a bug signal.

Keep a small, human-checkable suite whose optima can be independently derived, alongside randomly generated but seed-pinned fixtures. Never use the same scenarios both to tune and claim generalization.

## Proposed promotion targets (targets, not achieved results)

| Gate | Target for v0.1 qualification |
|---|---|
| Hard constraints | Zero accepted solutions with a hard-constraint violation across the qualification suite. |
| Tiny exact cases | 100% verifier agreement with hand-checkable/reference-oracle feasibility; where optimality is proven, normalized objective gap ≤ 1e-6 or the solver's justified tolerance. |
| Conservation | All ledgers reconcile within fixture-declared tolerances; zero double allocations in accepted results. |
| Reproducibility | Re-run of pinned inputs/configuration yields the same canonical plan and receipt digest, or documents the exact source of nondeterminism. |
| Held-out economic value | Stretch target: ≥5% median reduction in fully loaded cost per accepted unit against the strongest feasible conventional baseline on held-out cases, with no breach of service/safety/quality floors. Report all cases, including losses. |
| Tail resilience | Report worst-decile/CVaR-style loss and service floor under paired disturbances; no promotion based on mean cost alone. Quantify any intended cost–resilience trade-off. |
| Generalization | At least 30 tiny oracle cases and 50 held-out seeded scenarios across distinct price, availability, grade, and coupling regimes before claiming benchmark-level generalization. |
| Economic participation | Publish per-participant outcomes; any proposed cooperative policy must state which parties lose, why, and whether explicit transfer payments make them whole. |
| Comparative fairness | Matched information and compute budgets, logged seeds/configurations, baselines, ablations, negative controls, and a pinned independent verifier. |

The 5% cost target is an aspirational go/no-go target for this specific suite, not a universal claim about industrial savings. First publish the baseline distribution and variance; revise a target only with an explicit rationale. If the strongest baseline already reaches the optimum on a case, the correct result is parity, not fabricated novelty.

## Cross-project boundary contracts

- **Symtropy:** deterministic process/schedule/thermal simulation and candidate-plan evaluation; domain kernels state their valid regimes.
- **Symthaea:** optional hypothesis generation, search guidance, anomaly interpretation, and replanning policies. Evaluate against non-Symthaea arms under matched budgets.
- **Sol-Atlas:** evidence-qualified facility/resource capacity, spatial links, transport and utility context, plus source coverage and uncertainty. A map feature is not itself a verified capacity.
- **Mycelix:** versioned execution/lot provenance, authorization and exchange agreements, acknowledgements, and benefit-allocation records. It is not the real-time machine safety controller.
- **Independent evaluator:** verifies feasibility, conservation, scenario identity, objective recomputation, and output receipt. No participant's optimizer is its own sole auditor.

Start with versioned file contracts and adapters; do not require a distributed runtime to execute the deterministic offline benchmark.

## Benchmark stack and digital-twin composition contract

Do not evaluate a new industrial optimizer only on its own synthetic problem, and do not require every project to collapse into one schema or runtime. Qualify three layers separately:

1. **Component-method layer.** Use recognized external instances to test individual schedulers, and pinned analytic/public references to test process models. Report the component's own validity and solver-quality metrics before measuring network-level value.
2. **Composition layer.** Use the ISO 23247 digital-twin framework as the interoperability reference. ISO 23247-4 covers information exchange; ISO 23247-5 covers digital threads; ISO 23247-6:2026 explicitly addresses composition of manufacturing twins and distinguishes integrated, unified, and federated compositions. This project should define small, versioned composition profiles and test them rather than invent another incompatible monolithic twin format.
3. **Outcome layer.** Use INDUSTRIAL-COUPLING-001 to measure the marginal value (or cost) of combining components under physical constraints, uncertainty, participant economics, and independent verification.

### Composition profiles to test

- **Integrated:** use one shared schema only where the component owners have agreed on common semantics and a common lifecycle.
- **Unified:** keep each component's own model and schema; expose a composed view through explicit adapters with declared unit conversion, version, loss/coverage, and provenance.
- **Federated:** keep models and source data under separate ownership; exchange only the authorized contract fields and receipts required for coordination. Treat missing, stale, revoked, or inaccessible evidence as unknown/unqualified—not as zero or an assumed default.

These profiles are alternatives to compare, not a claim that one topology is always superior. Choose the narrowest composition that meets the workload and data-rights requirements. Symtropy should not import Sol-Atlas or Mycelix's internal domain logic; Symthaea should be tested as a consumer/producer of bounded decision proposals, not given implicit authority over a physical twin.

### Composition-level benchmark gates

- **Semantic loss:** zero silent drops, unit coercions, or enum remapping for decision-critical supported fields. Any unsupported field must be explicitly preserved as unknown or reported as unsupported.
- **Provenance:** 100% of decision-critical inputs bind to a source identity, source time, parser/adapter version, and qualification state. A profile match is not the same as a cryptographic source-identity check.
- **Freshness and gaps:** stale observations and large event gaps remain visible after composition; no adapter may promote event silence to machine availability.
- **Fault isolation:** unavailable or incompatible component evidence must yield an explicit degraded/unqualified composition, not a successful-looking zero/default.
- **Replaceability:** swapping a component implementation or version must not alter another component's authoritative source records; changes in composed outputs must be attributable to adapter/model version changes.
- **Reproducibility:** pinned component versions and input digests reproduce the same canonical composed request/receipt, or explicitly record the permitted source of nondeterminism.
- **Interop cost:** report adapter implementation effort, conversion/runtime overhead, payload size, end-to-end composition latency and the proportion of required fields actually observed.
- **Privacy/authority:** report which participant owns each field, which party may read or transform it, and how authorization/revocation affects cached and composed claims.

### Component benchmark preflight

A 2026 open-access job-shop benchmark paper describes publicly available environments and instances for classic job shop (JSP), flow shop (FSP), flexible job shop (FJSP), assembly job shop (AJSP), sequence-dependent setup times, and online job arrivals, spanning heuristic, exact, metaheuristic and reinforcement-learning methods. Use these published instances as an external preflight for scheduling algorithms where semantics match. Pin the exact release/commit and instances; record the known optimum or best-known value, solver time, compute budget and optimality gap. Extend the benchmark with industrial-specific energy, quality, evidence, maintenance and multi-party constraints only after recording the baseline result. Never claim that an integrated synthetic case demonstrates state-of-the-art job-shop scheduling.

For heat integration, use OpenPinch as an external reference candidate for the supported pinch/total-site calculations; a target calculation is not itself a validated heat-exchanger network. Keep external reference tools in the research/qualification path rather than adding a production Python runtime merely to conform to this benchmark.

## Phased work

1. **Contract:** freeze canonical schema, units, scenario IDs, score accounting, hard constraints, and negative controls.
2. **Reference evaluator (initial slice committed on this branch):** the standalone Rust heat-exchange screening crate validates input/output accounting and exactness for its bounded discrete model. Next, expand beyond this narrow slice only with hand-checkable fixtures; no external service or GPU requirement.
3. **Baseline tournament:** implement rule/independent and exact tiny cases first; then add a strong scalable conventional optimizer.
4. **Cross-project adapters:** ingest versioned evidence from owners above without copying their domain logic or confusing evidence classes.
5. **Robust evaluation:** paired disturbances, held-out regimes, ablations, sensitivity and per-participant economics.
6. **Pilot path:** only after independent qualification, run shadow recommendations at a consenting partner site; compare against measured baseline before any operational control.

No phase authorizes direct writes to operational technology. Any future plant deployment requires a site-specific hazard analysis, safety controller/interlocks, operator approval, rollback plan, and cybersecurity review.

## Research anchors

- NIST, [Smart Manufacturing Systems Test Bed](https://www.nist.gov/laboratories/tools-instruments/smart-manufacturing-systems-sms-test-bed): manufacturing data, MTConnect streams, data packages, and digital-thread interoperability are useful reference sources for later evidence adapters.
- Bottecchia, Kranzl & Zambelli (2024), [Driving industrial symbiosis: waste-heat utilization in local district-heating networks](https://doi.org/10.1016/j.enbuild.2024.114865): evaluates the economic conditions required for industrial heat-sharing arrangements.
- Applied Energy (2025), [Inter-plant heat exchanger network integration with an organic Rankine cycle](https://doi.org/10.1016/j.apenergy.2025.126126): a concrete example of the network-design and shared-cost-allocation problem; published findings are case-specific and should be reproduced rather than generalized.
- [OpenPinch](https://github.com/waikato-ahuora-smart-energy-systems/OpenPinch): an open-source process-stream integration implementation and potential pinned reference adapter. The existing Symtropy thermal work records a candidate multiperiod fixture; numeric parity must not be claimed until it is actually reproduced under a pinned environment.
- [Robust optimization for flexible industrial energy scheduling in a cement plant](https://doi.org/10.1016/j.egyr.2025.11.044): a recent scenario-based approach to electricity-price uncertainty and market participation, useful for baseline design. Its published outcomes are case-specific and should not be transferred to another facility without reproduction.
- [ISO 23247-6:2026 — Digital twin composition](https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3A23247%3A-6%3Aed-1%3Av1%3Aen): the current manufacturing digital-twin composition reference identifies integrated, unified and federated modes and describes requirements for composition and interoperation.
- Reijnen et al. (2026), [Job shop scheduling benchmark: environments and instances for learning and non-learning methods](https://doi.org/10.1007/s10472-026-10007-3): open benchmark environments span JSP, FSP, FJSP, AJSP, sequence-dependent setup and online job-arrival variants, allowing component scheduling results to be separated from the added difficulty of industrial coupling.

## Exit criteria

The first implementation is ready for review when the fixture schema, hand-checkable cases, independent verifier, hard gates, negative controls, matched-budget protocol, and output receipt format are all present. A passing synthetic benchmark proves only the tested contract and code path—not physical plant fidelity, field savings, general intelligence, or readiness to control production.
