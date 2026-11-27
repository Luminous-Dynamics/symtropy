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

## Execution rules

- Freeze corpus and manifest before measuring. Hash raw inputs and canonical model separately.
- Keep a held-out test set; do not tune against it.
- Compare only identical model classes, integer domains, units, boundaries, objective vectors, hardware class and resource budgets.
- For MCF, preserve lower bounds, node supplies/demands, parallel edge identities and the exact lexicographic vector. No unproven Big-M equivalence.
- Record parsing, canonicalization, model construction, solve, proof generation, proof verification and end-to-end latency separately.
- Keep unsupported, indeterminate, DNF, timeout, false guarantee and false reject in the denominators and report each separately.
- Re-run after every code, schema, model or comparator change. Queued/skipped/cancelled CI is not qualification.
- Never allow a schema-valid artifact to be called a valid certificate without independent cryptographic/domain verification.

## Acceptance and reporting

The top-level SAFRB protocol contains the prospective false-guarantee, positive-control, exact proof, latency and scale targets. This manifest supplies per-case traceability to those targets; it does not aggregate them into one score. A benchmark result is accepted only when the exact manifest, inputs, run receipt and independent verification can be replayed.

## Relationship to existing work

Composes with PR #7375 (service assurance vector and matched ablation), PR #7591 (flow certificate), PR #7597/#7749 (temporal semantics), PR #7736/#7710 (lexicographic MCF and optimality proof), PR #7762/#7761 (coherent scenario semantics), #7760 (mandatory beneficiary floors), and #7763 (read-only vendor-neutral integration). Ownership boundaries remain with their existing contracts.

## Schema

`assurance-benchmark-manifest-v0.1.schema.json` is JSON Schema Draft 2020-12 and rejects undeclared top-level, nested and comparator properties. Schema validation is only input-shape validation; it does not authenticate URI contents, issuer authority, expected results or certificate correctness.
