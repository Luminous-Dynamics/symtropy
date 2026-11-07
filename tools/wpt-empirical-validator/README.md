# WPT empirical dataset validator

This Rust workspace tool performs an offline integrity check of
`benchmarks/wpt-empirical/v0/`.

It validates the source and realization JSONL records against the required
fields declared in their committed JSON Schemas, then applies additional
cross-record and semantic checks:

- unique source IDs, system IDs, per-system measurement IDs and engineering
  metric IDs;
- all realization source references resolve to the source catalog;
- schema-required top-level and measurement fields are present;
- evidence kinds, test modes, source roles, rights fields and claim dispositions
  use supported values;
- power and efficiency qualifiers agree with their numeric values;
- range-valued outputs have ordered lower/upper bounds;
- efficiency stays in the 0–100 percent range;
- unreported values remain null rather than silently becoming zero.

This is a focused integrity validator, not a complete implementation of the
JSON Schema 2020-12 specification. JSON Schema files remain the authoritative
record contracts; this tool enforces the current contract's required fields and
WPT-specific semantic invariants. It does not adjudicate scientific truth,
measurement validity, rights clearance, regulatory compliance or whether two
papers describe independent physical prototypes.

## Run

From the repository root:

    cargo test -p wpt-empirical-validator --locked
    cargo run -p wpt-empirical-validator --locked

An optional first argument supplies an alternate repository root:

    cargo run -p wpt-empirical-validator --locked -- /path/to/symtropy

A successful run prints source, prototype, measurement and engineering-metric
counts. A malformed record, unresolved source ID, duplicate identity, invalid
qualifier or range inconsistency exits non-zero.

## Validation limits

The validator checks structural and consistency rules only. It does not prove
the authors' interpretation is correct, that a measurement is independently
reproduced, that an efficiency boundary is physically comparable, or that a
reported safety result establishes legal compliance. Those require source
review, expert annotation and task-specific independent verification.
