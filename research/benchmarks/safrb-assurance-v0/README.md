# SAFRB Assurance Benchmark v0 — preregistration

**Status:** research protocol; prospective targets are not achieved results.
**Scope:** benchmark the evidence-to-service claim chain, not only the network-flow solver.

## Central hypothesis

A trustworthy infrastructure assurance system must prove more than that an encoded graph has a maximum flow or minimum-cost solution. It must preserve the semantics of the service obligation, model exact resource/time constraints, bind evidence and authority to those inputs, independently verify the resulting allocation, and clearly state what is not established.

The proposed competitive differentiator is a **vendor-neutral, evidence-linked service claim receipt**. It can be consumed by an operator or digital-twin platform but does not replace its control loop or authorize action.

## Four non-compensatory benchmark lanes

### 1. Solver lane — mathematical performance

Use the 13th DIMACS Network Flows 2.0 corpus and generators. Keep maximum flow and minimum-cost flow as separate problem classes. For MCF, compare LEMON CostScaling and NetworkSimplex and Google OR-Tools SimpleMinCostFlow; use NetworkX as a secondary bounded-instance differential oracle. Preserve parallel-arc identity, lower bounds, supply/demand conventions, integer ranges and the exact lexicographic objective vector. Never compare solvers on transformed objectives that are not mathematically equivalent.

Official references:
- DIMACS Network Flows 2.0: https://coral.ise.lehigh.edu/flow-challenge-2-0/
- MCF instances and generators: https://coral.ise.lehigh.edu/flow-challenge-2-0/mcf-instances-and-generators/
- MCF format: https://coral.ise.lehigh.edu/flow-challenge-2-0/minimum-cost-flow-input-format/
- LEMON CostScaling: https://karganys.github.io/lemon/a00109.html
- Google OR-Tools SimpleMinCostFlow: https://developers.google.com/optimization/flow/mincostflow
- NetworkX network_simplex: https://networkx.org/documentation/stable/reference/algorithms/generated/networkx.algorithms.flow.network_simplex.html

DIMACS explicitly notes that older instances may be too easy for modern algorithms. It also published a September 26, 2026 GENRMF generator overflow fix after negative capacities appeared in large instances. Therefore, benchmark corpus provenance, importer correctness, integer-boundary rejection and generator integrity are part of the evaluation—not just solver runtime.

### 2. Model lane — semantic fidelity

Use frozen physical scenarios and independent physical-quantity oracles, not only differential agreement between graph solvers. Required families include:
- Cross-dock throughput above end-of-period storage.
- Storage-bound carry and handling-bound dispatch, independently.
- Shared handling gates across parallel departures and resource aliases.
- Start-of-period receipt versus stored/staged receipt admission.
- Route transit, event ordering, half-open intervals and deadline boundaries.
- Per-beneficiary request ceilings versus explicit mandatory minimum floors.
- Mutually exclusive scenario maxima that must not be spliced into a fictitious operating mode.

Require exact exhaustive parity on tiny integral cases and seeded differential corpora. Unsupported multi-commodity, process-coupled, stochastic, hydraulic or integer-scheduling cases remain UNSUPPORTED unless the correct model and solver class is used.

Current research anchors: Symtropy #7735 (temporal semantics), PR #7749 (v0.1 compiler and independent verifier), #7751 (dynamic evidence-bound capacity and common resources), #7760 (beneficiary floors and priorities), #7761 (coherent joint capacity scenarios), PR #7762 (scenario coherence prototype).

### 3. Evidence lane — source, authority and common cause

Construct cases where the encoded graph is mathematically solvable but its inputs are not decision-admissible: stale/revoked measurements, wrong units or boundaries, unsupported capacity class, altered source revision, missing issuer authority, duplicate dashboards backed by one sensor, split resource aliases, incomplete dependency closure, and component-wise upper bounds from mutually exclusive modes.

Evidence admission and legal rights belong to the appropriate trust/provenance service (for this programme, Mycelix), not to the flow solver. The solver must consume an explicit admitted profile and preserve its references; it must not self-authenticate them.

Relevant measurement/provenance references:
- NIST TIDAL / Super Intelligence for Manufacturing: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
- NIST IR 8536, Supply Chain Traceability Meta-Framework (final September 9, 2026): https://csrc.nist.gov/pubs/ir/8536/final
- NIST Digital Twins Workshops Summary Report, NISTIR 8620 (July 21, 2026): https://www.nist.gov/publications/digital-twins-workshops-summary-report

NIST TIDAL focuses on layered industrial intelligence, human-machine teaming, contextualized data, integration benchmarks and digital-twin VVUQ. NIST IR 8536 addresses linked, cryptographically verifiable provenance records and selective disclosure across supply chains. These are assurance and interoperability reference points, not competing MCF solvers.

### 4. Operational lane — integration and outcomes

Do not claim physical qualification from synthetic flow. Progress through:
1. Frozen synthetic/known-answer fixtures.
2. Deterministic replay of owner-authorized historical records.
3. Software-in-the-loop with the source platform or digital twin.
4. Hardware/controller-in-the-loop.
5. Operator-approved site commissioning, controlled read-only observation and periodic requalification.

Potential integration classes include AVEVA PI System / PI Data Infrastructure (industrial data collection and contextualization), Siemens Gridscale X (grid planning, operations, maintenance and digital-twin workflows), and Schneider EcoStruxure Microgrid Advisor (DER monitoring, forecasting and model-predictive optimization). These are not interchangeable products or direct solver competitors. The test is whether the assurance receipt composes with their data/operations interfaces without replacing their operator authorization or control plane.

Vendor references:
- AVEVA PI System: https://www.aveva.com/en/products/aveva-pi-system/
- Siemens Gridscale X: https://www.siemens.com/en-us/products/gridscale-x/
- Schneider EcoStruxure Microgrid Advisor: https://www.se.com/us/en/product-range/65896-ecostruxure-microgrid-advisor/

## Prospective acceptance targets

These are proposed gates, not achieved results.

| Dimension | Prospective target | Reporting rule |
|---|---:|---|
| False guaranteed-service verdicts | 0 / 300 independent negative cases | One-sided 95% binomial upper bound is about 1% when zero events are observed; publish the exact interval |
| Supported-positive controls | At least 99% acceptance across 300 controls | Publish confidence interval and every false rejection |
| Valid proof receipts | 100% accepted | Independent verifier, exact graph/model/schema digest |
| Mutated proof receipts | 100% rejected for each defined mutation family | No mutation class hidden inside one aggregate score |
| Exhaustive tiny solver/model cases | 100% exact result parity | Integral flow/objective and physical feasible-set oracle |
| Decision-path verification | P95 verifier latency below 10% of the declared decision-time budget | Also report P50/P99 and deadline misses |
| Admitted scale Stage A | 100k nodes / 1M arcs | No OOM within declared budget; no truncation or capacity loss |
| Admitted scale Stage B | 1M nodes / 8M arcs | Preflight resource ceilings; full certificate/verifier parity |

Do not use one weighted benchmark score. Report at least:
- parse/import and input-integrity time;
- model compilation and canonical digest time;
- graph construction and solve time;
- certificate construction and serialization time;
- independent verification time;
- peak RSS and declared CPU/memory budget;
- objective parity and feasibility;
- false guarantees, false rejects, unsupported, indeterminate, DNF and timeout counts;
- end-to-end decision latency and actual beneficiary outcome where measured.

Decision-quality outcomes should include unmet beneficiary floors, service regret against a frozen oracle, false-positive guarantee rate, false-negative guarantee rate, intervention frontier changes and cost/energy/water/carbon only where the input units and valuation authority are explicit. Solver dual values are not tariffs, market prices or social value.

## Preregistration and reproducibility

Before running a benchmark, freeze the input corpus and manifest; publish each artifact digest, model class, objective vector, unit/boundary, source revision, license, generator/version/seed, toolchain, hardware, memory ceiling, warm-up/repetition policy and timeout. Keep a held-out set that is not used for tuning. A run is comparable only if the same model, data, objective, hardware class and budget are used.

Every result must point to an exact repository commit. A later commit requires a fresh receipt. Queued, skipped, cancelled or stale-head workflows are not passes.

The companion JSON Schema is `manifest.schema.json`. It is designed to keep raw artifact identity, canonical model digest, expected results, mutations and resource budgets distinct.

## Current evidence snapshot (November 27, 2026)

- Shared single-commodity kernel PR #7591: exact-head formatting, Clippy, tests and portable certificate conformance passed on `56d007c38f6ed27cbe0744c80531e2dc25668245`.
- SAFRB temporal adapter PR #7597: exact-head formatting, Clippy, tests and portable certificate conformance passed on `e869faa9dd66ee481af58bc9d9325e92b6c16a6a`.
- Lexicographic MCF conformance PR #7736: exact-head workflow passed on `f1d40c37b6f195ba04d53802717e7c93aa0bd348` with 1,000 seeded small instances and 100 parallel-arc identity cases. This is objective-transformation conformance, not a Rust planner performance result or SOTA win.
- Temporal profile PR #7749: exact-head workflow passed on `97757acff2e9c9e2cb507e574ec3865cbc887a0e`, including 250 seeded physical-oracle cases and independent temporal certificate checks. This validates only the declared synthetic single-commodity profile.
- Coherent scenario PR #7762: research implementation and schemas are in progress; its latest exact-head run must pass before any qualification claim.

No industry SOTA performance advantage, source authority qualification, physical asset qualification, dispatch authority or real-world beneficiary-service result is claimed by this snapshot.