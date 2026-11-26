# Single-commodity directed flow kernel

This crate provides a deterministic integer-capacity max-flow/min-cut kernel for already validated, single-commodity directed networks. It is an algorithm component, not a domain model.

## Contract

- Stable, unique node and edge identities.
- One opaque commodity reference, capacity-unit reference and common quantity-boundary reference per solve; all three are included in the canonical graph digest and returned certificate.
- Non-negative integer capacities (u64); no floating-point feasibility tolerances.
- Directed arcs; parallel and antiparallel edges are valid.
- Deterministic node/edge ordering and tie-breaking.
- Constructive edge-level flow witness and residual source-side partition.
- Minimum-cut edge identities, capacities and exact max-flow/min-cut equality.
- Versioned canonical graph encoding and SHA-256 binding.
- Portable graph-bound certificate bundle (`FlowCertificateBundle`) with strict Serde decoding, a published JSON Schema and an independent post-deserialization verifier.
- Explicit graph-size bounds and checked integer arithmetic.
- Independent certificate verification without rerunning the solver, including exact reconstruction of residual reachability from the submitted edge-flow witness; a substituted equal-capacity cut partition is rejected.

The canonical graph digest binds commodity, capacity unit, quantity boundary, source/sink IDs, node IDs, edge IDs, endpoints, capacities and opaque metadata references. Input permutation does not change the digest.

## Deliberate non-goals

The kernel does not infer units, time chronology, storage carry-over, route delay, process conversion, physical resource identity, alias equivalence, evidence freshness, calibration, source dependence, title, custody, contract access, legal authority or dispatch permission.

A domain adapter must collapse logical aliases onto physical-resource capacity gates before constructing the graph. Lower and upper capacity envelopes are separate graph instances. An optimistic upper-bound solution is not a guarantee.

Multi-commodity flow, integer scheduling, process-coupled LP, nonlinear constraints and shared constraints without a valid network representation are unsupported by this kernel. Callers must return an explicit unsupported/not-evaluable disposition or use a separately qualified solver and certificate type.

## Licensing

AGPL-3.0-or-later. The package is added as an explicitly AGPL-licensed extraction in the Symtropy workspace; this change does not relicense the existing WPT oracle or any dependent project.

## Validation

The unit suite exhaustively enumerates all simple directed graph topologies through four nodes and compares against an independent source/sink cut enumerator. A second deterministic differential corpus checks 1,120 generated graphs across 2–8 nodes against the independent cut oracle and runs the certificate verifier on every result. It also tests parallel and antiparallel arcs, cycles, zero capacity, reverse-residual rerouting, `u64::MAX`, disconnected graphs, malformed identities/endpoints, overflow, permutation invariance, forged equal-capacity cuts, strict portable serialization and graph/certificate substitution.

See [FLOW_CERTIFICATE_V1.md](FLOW_CERTIFICATE_V1.md) for the portable bundle contract and [the JSON Schema](schemas/flow-certificate-v1.schema.json) for the wire shape.

These tests establish algorithm behavior for the declared graph model only. They do not qualify physical inputs, infrastructure, production integrations or field service.
