# Portable flow certificate v1

The kernel exports a **proof-carrying bundle**, not just a scalar maximum-flow value:

- `graph`: the complete declared directed single-commodity graph, including IDs, endpoints, capacities, quantity contract and opaque domain metadata references;
- `result`: graph digest, algorithm/model version, edge-level primal flow, residual-reachable source-side nodes, cut-edge identities and cut capacity;
- `schema_version`: explicit wire-format version.

The Rust type is `FlowCertificateBundle`. Create it with `FlowCertificateBundle::new(graph, result)`; deserialize it with Serde; then call `bundle.verify()`. Both operations check graph/result binding and exact certificate invariants. Serde rejects unknown fields at every nested struct. JSON Schema: `schemas/flow-certificate-v1.schema.json`.

## Exact graph digest

The graph digest is SHA-256 over the following canonical byte encoding. It is intentionally independent of JSON property order and input array order.

1. Append the 22 ASCII bytes `LUMINOUS-SCFK-GRAPH-V1` followed by one NUL byte.
2. Encode each string as an unsigned 64-bit little-endian byte length followed by that many UTF-8 bytes.
3. Encode each unsigned 64-bit integer as eight little-endian bytes.
4. Append, in order: `commodity_ref`, `capacity_unit_ref`, `quantity_boundary_ref`, `source`, `sink`.
5. Append the node count, then each node ID sorted in ascending UTF-8 byte order.
6. Append the edge count, then for each edge sorted by edge ID: edge ID, from-node ID, to-node ID, capacity, then one metadata tag byte. Tag `0` means no metadata reference; tag `1` is followed by the metadata-reference string.

The digest is rendered as `sha256:` plus 64 lowercase hexadecimal characters. The Rust implementation exposes `FlowGraph::canonical_bytes()` so independent implementations can test byte-for-byte parity.

## Verification requirements

A consumer must not trust a certificate because its JSON parses or its hash field has the right shape. It must:

1. Recompute canonical graph bytes and SHA-256 from the supplied graph.
2. Check schema, kernel version, supported model class, commodity, unit, boundary, source and sink binding.
3. Check every original edge occurs exactly once in the flow witness; endpoints, capacity and metadata must match; flow must not exceed capacity.
4. Check flow conservation at every vertex, with `max_flow` as the source outflow and sink inflow.
5. Reconstruct forward and reverse residual arcs from each submitted edge flow, then recompute the exact residual-reachable source side.
6. Recompute the cut edge identities/capacities from that source side and check cut capacity equals max flow.

This rejects a different equal-capacity cut partition, an omitted edge, duplicated edge, swapped endpoint, capacity mutation, forged maximum, incorrect unit/boundary or altered graph.

## Integer interoperability

All capacity and flow quantities are Rust `u64`, represented in JSON as integer tokens. Consumers must parse them as exact unsigned 64-bit integers, **never binary floating-point**. A JSON parser that rounds integers above 2^53 is not suitable for verification. Values outside `0..=18446744073709551615` must be rejected.

## Claim ceiling

The bundle proves a maximum-flow result for the exact graph presented. It does not authenticate the graph's evidence, prove that metadata references exist, establish physical identity, physical capacity, rights, ownership, legal authority, fairness, field service or authorization to act. Those remain domain-owned predicates and attestations.
