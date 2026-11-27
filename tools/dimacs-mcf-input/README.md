# DIMACS minimum-cost-flow input

A bounded, standard-library-only Rust parser and deterministic serializer for the DIMACS Network Flows 2.0 minimum-cost-flow input format.

This crate is an ingestion and identity-preservation layer. It is **not** a minimum-cost-flow solver, feasibility oracle, optimality certifier, or model-authentication system.

## Supported records

- `p min n m`: one problem header with node and arc counts.
- `n vertex supply`: optional nonzero signed supply records, before all arcs.
- `a tail head lower upper cost`: exactly the declared number of arc records.

Comments (`c` records), blank lines, CRLF, and a final line without a newline are accepted. Record field counts are exact. Node IDs are one-based. The first arc endpoint is the tail/source vertex; the second is the head/destination vertex.

Numeric representation is preserved without narrowing:

- node supplies: signed 64-bit integers;
- arc lower/upper bounds: signed 64-bit integers, validated as nonnegative with upper >= lower;
- arc unit costs: signed 32-bit integers;
- problem node/arc counts: limited to the DIMACS signed-31-bit positive range.

Parallel arcs remain separate records with stable one-based ordinals in their original input order. They are never summed, overwritten, or collapsed by endpoint pair. This matters for distinct routes, evidence lineage and per-arc certificate reconstruction.

A structurally valid instance with an unbalanced supply vector is preserved as parsed input. `McfInstance::supply_balance()` reports the signed sum using i128; `is_supply_balanced()` is a convenience predicate. The parser does not rewrite the problem or pretend that imbalance is feasible.

## Resource limits

Default budgets:

| Resource | Default |
|---|---:|
| Input bytes | 256 MiB |
| Single line | 16 KiB |
| Nodes | 100,000 |
| Arcs | 1,000,000 |

Use `parse_dimacs_mcf_with_limits` to set a different explicit budget for a benchmark or controlled import. Header declarations are checked before allocating node/arc vectors; vector reservations use `try_reserve_exact`. Over-budget input is rejected rather than silently truncated.

Example:

```rust
use dimacs_mcf_input::{
    parse_dimacs_mcf_with_limits, McfParseLimits,
};

let limits = McfParseLimits {
    max_input_bytes: 512 * 1024 * 1024,
    max_line_bytes: 16 * 1024,
    max_nodes: 1_000_000,
    max_arcs: 8_000_000,
};

let instance = parse_dimacs_mcf_with_limits(dimacs_text, limits)?;
assert!(instance.is_supply_balanced());
```

## Canonical reserialization

`McfInstance::to_dimacs()` emits one deterministic representation:

1. one problem header;
2. nonzero node supplies in ascending vertex order;
3. arc records in original input order.

The original arc ordinal and exact endpoints, bounds and costs are retained. The source line is diagnostic metadata and is not part of semantic arc equality or the serialized representation. Reparse and compare to test semantic round trips.

## Independent primal/dual optimality certificates

The certificate module verifies an original-space min-cost-flow witness without calling a solver. A certificate carries:

- the exact certificate schema version;
- SHA-256 of the canonical DIMACS instance;
- one flow record per original arc, including its one-based ordinal;
- one i128 node potential per vertex, with the zero sentinel at index zero;
- the exact i128 objective value.

The verifier checks graph binding, arc identity/order, lower and upper bounds, node conservation under the positive-supply convention, exact objective reconstruction, and dual reduced-cost conditions. For each arc u -> v with unit cost c and node potentials pi:

reduced_cost(u -> v) = c + pi[u] - pi[v].

Every available forward residual arc (flow < upper) and reverse residual arc (flow > lower) must have nonnegative reduced cost. Together with primal feasibility, this rules out negative-cost residual cycles and proves global optimality for the admitted single-commodity linear network instance.

The certificate is checked in original DIMACS arc space. The exact original lower/upper bounds and per-arc costs remain visible, and a changed parallel-arc order changes the canonical digest. The verifier does not trust a solver status string or compare two solvers' objective values as a substitute for a proof.

Example:

~~~rust
use dimacs_mcf_input::{
    canonical_instance_sha256, verify_mcf_optimality,
    McfArcFlow, McfOptimalityCertificate,
    MCF_OPTIMALITY_CERTIFICATE_SCHEMA,
};

let certificate = McfOptimalityCertificate {
    schema_version: MCF_OPTIMALITY_CERTIFICATE_SCHEMA.to_string(),
    graph_digest: canonical_instance_sha256(&instance)?,
    arc_flows: vec![
        McfArcFlow { ordinal: 1, flow: 3 },
    ],
    node_potentials: vec![0, 0, 7],
    objective_cost: 21,
};
verify_mcf_optimality(&instance, &certificate)?;
~~~

The verifier is an independent check, not a certificate generator or MCF optimizer. Solver adapters still need to obtain primal flows and dual potentials, serialize them with the instance digest, and compare objective tuples only when objectives are mathematically identical. It also does not turn multi-commodity, integer scheduling, nonlinear or process-coupled models into a single-commodity MCF problem.

## Deliberate boundaries

This module does not:

- solve min-cost flow or determine feasibility/optimality;
- interpret lower bounds as an already feasible flow;
- infer units, currencies, physical capacity, evidence authority or time semantics;
- convert multi-commodity, integer-scheduling, nonlinear or process-coupled systems into single-commodity MCF;
- certify solver output.

In particular, solver adapters must implement the exact DIMACS lower-bound-to-balance transformation, preserve parallel-arc identity, and return a witness in the same logical arc space. The parser does not choose a solver or alter the cost objective.

## Benchmark use

Use the same parsed instance and canonical graph identity for each solver. Keep these measurements separate:

- input bytes read and parsed;
- parser and canonicalization time;
- solver-specific model build and lower-bound transformation;
- solve time;
- primal witness production;
- dual/reduced-cost certificate production;
- independent certificate verification;
- peak RSS, DNF, timeout and unsupported-case counts.

Do not compare maximum-flow against minimum-cost-flow results, or compare scalar Big-M objectives against sequential lexicographic passes as though they were the same problem.

Planned reference portfolio:

- LEMON CostScaling and NetworkSimplex as primary DIMACS MCF references;
- OR-Tools SimpleMinCostFlow as an independent production-grade comparator;
- NetworkX capacity_scaling and network_simplex for bounded differential and exhaustive tests.

DIMACS Network Flows 2.0 has official instance families and generators. Some historical instances are now too easy for current algorithms, so report results on the official families plus fresh seeded, adversarial and held-out instances. Do not claim a state-of-the-art improvement from parser correctness or tiny objective-conformance tests.

## Tests

The parser unit suite covers signed integer boundaries, input order, parallel arcs, source/tail orientation, unbalanced supplies, CRLF/comments, canonical round-trip, duplicate headers and supply records, invalid bounds, invalid endpoints, malformed record counts, missing and extra fields, and pre-allocation byte/line/node/arc budgets.

The repository-pinned Rust exact-head workflow includes this crate's formatting, Clippy and test gates. A successful result is evidence only for the exact source SHA recorded in its workflow run.
