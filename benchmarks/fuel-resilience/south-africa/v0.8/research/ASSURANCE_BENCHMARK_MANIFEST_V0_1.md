# SAFRB assurance benchmark manifest v0.1

**Purpose:** make each benchmark case reproducible, independently interpretable and difficult to game.
**Status:** preregistration contract; no new performance or field qualification claims.

This manifest extends the existing [service-assurance vector and matched benchmark protocol](SERVICE_ASSURANCE_VECTOR_AND_BENCHMARK_PROTOCOL.md). It does not replace that protocol, existing SAFRB schemas, the Power Fabric receipts, source verification or owner-specific evidence contracts.

## Why a manifest is needed

A solver runtime without an exact input digest, model profile, objective definition, resource budget and independent oracle is not a reproducible comparison. Likewise, two solvers that agree on a graph do not establish that the graph faithfully represents a physical service chain.

The manifest keeps the following identities separate:

1. **Raw input artifact digest** — exact source bytes / frozen fixture.
2. **Canonical model digest** — exact normalized graph or domain model after a versioned compilation step.
3. **Objective digest** — ordered objective components, directions and units.
4. **Comparator configuration digest** — exact solver version and options.
5. **Run receipt digest** — output artifacts, toolchain/hardware, timing, memory, disposition and verifier outcomes.

Do not substitute one digest for another. A changed parser, schema, timebase, unit, boundary, capacity class, scenario relation or objective changes the relevant model identity and requires a fresh run.

## Required registration

Each case records its benchmark lane, source URI and license, source revision/generator/seed, raw artifact digests, model class/schema/commodity/unit/boundary/timebase, ordered objective, comparator versions/configurations, independent oracle kind, CPU/memory/wall-clock limits, mutation families and claim ceiling.

Supported lanes:
- `solver`: max-flow or min-cost-flow mathematical comparison.
- `model_semantics`: temporal and physical conservation/compiler fidelity.
- `evidence_integrity`: issuer, provenance, freshness, authority, common-cause and alias attacks.
- `coherent_scenarios`: scenario splice, exclusivity and robust-versus-possible separation.
- `service_outcomes`: beneficiary floors and usable-service disposition.
- `interop`: vendor-neutral source/receipt fidelity and read-only behavior.

## Non-anticipativity and recourse comparison

Treat scenario-specific full recourse and a non-anticipative policy as different model classes. A full-recourse evaluation may choose a different allocation after each scenario is observed. A here-and-now or staged policy must choose identical actions for scenario histories that are still observationally indistinguishable.

For every non-anticipative-policy case, freeze and digest:
- the scenario tree and every leaf/path;
- the observation/revelation schedule and each information-set partition;
- each decision stage and whether it is binding before or after an observation;
- the action variables committed at each information set;
- pathwise resource budgets and shared-resource dependencies.

Required comparator classes for small, exact cases: exhaustive policy enumeration; deterministic-equivalent MILP with explicit non-anticipativity equalities; full-recourse scenario-specific oracle as an information relaxation; and a clairvoyant per-leaf bound for diagnostics only. For larger cases add matched progressive-hedging/scenario-decomposition, stochastic dual dynamic programming or integer extensions where their assumptions hold, rolling-horizon MPC, and decision-focused learning. Never compare algorithms under different recourse assumptions as though they optimize the same problem.

Key metrics: here-and-now feasibility, full-recourse feasibility, exact policy gap, worst-leaf regret, non-anticipativity violations, pathwise budget violations, information-set/tree size, solver gap/node count, policy inference latency, training and solve compute, and independent-verification overhead. Keep probability-weighted objectives separate and use them only when scenario probabilities are authoritative.

## Execution rules

- Freeze corpus and manifest before measuring. Hash raw inputs and canonical model separately.
- Keep a held-out test set; do not tune against it.
- Compare only identical model classes, integer domains, units, boundaries, objective vectors, hardware class and resource budgets.
- For MCF, preserve lower bounds, node supplies/demands, parallel edge identities and the exact lexicographic vector. No unproven Big-M equivalence.
- Record parsing, canonicalization, model construction, solve, proof generation, proof verification and end-to-end latency separately.
- Keep unsupported, indeterminate, DNF, timeout, false guarantee and false reject in the denominators and report each separately.
- Re-run after every code, schema, model or comparator change. Queued/skipped/cancelled CI is not qualification.
- Never allow a schema-valid artifact to be called a valid certificate without independent cryptographic/domain verification.

## Solver comparator matrix (November 2026)

Keep maximum flow and minimum-cost flow as separate lanes.

### Maximum flow

The shared Rust SCF kernel currently uses a Dinic-style level graph and blocking-flow algorithm. Its caps of 4,096 nodes and 65,536 arcs are deliberate admission limits, not a large-scale or SOTA claim. The DIMACS 2.0 reference page recommends HIPR4 (highest-level push-relabel), Pseudoflow and EIBFS; it names HIPR4 as the main cross-family baseline because no solver dominates every family. Benchmark exact same graphs and integer capacities, and measure where Dinic is competitive, slower, faster or unsupported. Preserve parser/format translation, graph digest, proof production and proof verification overhead. Reference: https://coral.ise.lehigh.edu/flow-challenge-2-0/max-flow-reference-solvers/

### Minimum-cost flow

Primary practical baselines remain LEMON 1.3.1 CostScaling and NetworkSimplex, plus Google OR-Tools SimpleMinCostFlow. NetworkX is a secondary small-instance differential reference; NetworkSimplex requires a directed simple graph, so any parallel-arc transformation must retain a verifiable map back to original arc IDs.

An important new research comparator is Schätzle, Meierhans, Kyng and Probst Gutenberg, *A Practical Combinatorial Interior Point Method for Minimum-Cost Flow*, arXiv:2610.18328 (submitted October 13, 2026): https://arxiv.org/abs/2610.18328. It compares a practical combinatorial IPM against LEMON on 12 DIMACS core families. The paper reports competitive scaling on five families and better scaling on two; on `grid_wide`, it reports approximately `m^1.04` against LEMON's `m^1.48`. The authors also report a feasible but non-optimal result on `grid_long`, and livelock/non-termination on a `road_flow` case under some precision settings.

Those results make the IPM an important emerging comparator, not an overall winner. The paper's experimental section reports one run on one seed per family/size pair on an AMD EPYC 7742 / 512 GB host, with C++23 `-O3 -march=native -ffast-math`; the observed results are informative but not a repeated-run leaderboard. It is a strong reason to record false-optimum, verifier-rejection, early-termination, livelock and timeout outcomes separately. The independent primal/dual verifier must catch a feasible-but-non-optimal result.

### Matched-comparison rules

- Compare the same integer network, node supplies/demands, lower/upper bounds, parallel arcs, time expansion and ordered objective vector.
- For lexicographic SAFRB objectives, apply identical exact sequential passes or a proven mathematically equivalent encoding; never compare different Big-M or scalarized objectives as if they were the same problem.
- Record source code revision and license, compiler/toolchain flags, CPU/memory, seeds, warmups, repetitions, timeout and warm-start policy.
- Report parse/model-build/solve/witness/dual-proof/independent-verification times separately.
- Reject or flag outputs with exact feasibility but incorrect objective; DNF, livelock, timeout and unsupported are separate outcomes.
- Keep solver score independent from evidence admission, model-semantic correctness and service-outcome scorecards.

## Acceptance and reporting

The top-level SAFRB protocol contains the prospective false-guarantee, positive-control, exact proof, latency and scale targets. This manifest supplies per-case traceability to those targets; it does not aggregate them into one score. A benchmark result is accepted only when the exact manifest, inputs, run receipt and independent verification can be replayed.

## Relationship to existing work

Composes with PR #7375 (service assurance vector and matched ablation), PR #7591 (flow certificate), PR #7597/#7749 (temporal semantics), PR #7736/#7710 (lexicographic MCF and optimality proof), PR #7762/#7761 (coherent scenario semantics), #7760 (mandatory beneficiary floors), and #7763 (read-only vendor-neutral integration). Ownership boundaries remain with their existing contracts.

## Schema

`assurance-benchmark-manifest-v0.1.schema.json` is JSON Schema Draft 2020-12 and rejects undeclared top-level, nested and comparator properties. Schema validation is only input-shape validation; it does not authenticate URI contents, issuer authority, expected results or certificate correctness.
