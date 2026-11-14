# mus-opt-audit

Standalone Rust validator for the MUS-OPT-001 instrument architecture matrix.

It checks the corpus against Draft 2020-12 JSON Schema and then validates domain-specific invariants that are awkward to express in the schema alone:

- complete 3 × 3 baseline coverage, exactly once per architecture/route pair;
- unique scenario identities and complete expected challenge coverage;
- fixed units/objective directions;
- unresolved metrics stay null and cannot masquerade as free/zero-cost evidence;
- quote/observed/derived metric values bind a non-empty source reference and an as-of date;
- currency code requirements for resolved BOM cost;
- baseline/stress/negative-control claim dispositions.

The report is limited to synthetic scenario consistency. It does not rank candidates, fetch supplier data, verify quotations, calculate real manufacturing costs, inspect calibration, verify a physical build, or grant hardware-release authority.

## Run

From the repository root:

```sh
cargo run --manifest-path tools/mus-opt-audit/Cargo.toml -- \
  --schema schemas/manufacturing/instrument_architecture_matrix_v0_1.schema.json \
  --corpus fixtures/manufacturing/instrument_architecture_matrix_v0_1.json
```

To run unit tests:

```sh
cargo test --manifest-path tools/mus-opt-audit/Cargo.toml
```

Exit codes: 0 = structurally/policy-consistent but still unqualified; 1 = schema or invariant rejection; 2 = I/O, parse, compilation, or CLI error.

Network/file reference fetching is disabled in the pinned JSON Schema validator configuration. The current corpus uses only local `#/$defs` references.

## Evidence boundary

A source ID in this corpus is only a reference string. This utility does not authenticate source identity, quote freshness, supplier availability, physical measurements, or reviewer authority. Those require separate resolvers and evidence policies.
