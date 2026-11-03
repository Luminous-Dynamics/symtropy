# WPT empirical dataset validator

This Rust workspace tool performs an offline integrity check of
\`benchmarks/wpt-empirical/v0/\`.

It validates source and realization JSONL records against every validation
constraint currently used by the committed JSON Schemas, then applies
additional WPT-specific and cross-record checks:

- JSON types, nullable unions, constants and enumerations;
- required properties and closed objects (\`additionalProperties: false\`);
- nested object properties and array item schemas;
- string patterns, minimum lengths and absolute URI format assertions;
- numeric inclusive/exclusive bounds and array minimum lengths;
- unique source IDs, system IDs, per-system measurement IDs and engineering
  metric IDs;
- all realization source references resolve to the source catalog;
- evidence kinds, test modes, source roles, rights fields and claim dispositions
  use supported values;
- power and efficiency qualifiers agree with their numeric values;
- range-valued outputs have ordered lower/upper bounds;
- efficiency stays in the 0–100 percent range;
- unreported values remain null rather than silently becoming zero.

## Schema vocabulary and fail-closed behavior

The current schemas use a deliberately limited JSON Schema 2020-12 vocabulary:
\`type\`, \`const\`, \`enum\`, \`required\`, \`properties\`,
\`additionalProperties\`, \`items\`, \`minItems\`, \`minLength\`, \`pattern\`,
\`format: uri\`, \`minimum\`, \`maximum\` and \`exclusiveMinimum\`.

The validator checks that all schema keywords are in this supported set (or
are non-validating annotations such as \`title\` and \`description\`). If a
future schema adds a validation keyword or format that the tool does not
implement, validation fails closed until support and tests are added. The URI
format is asserted by parsing an absolute URI; this is intentionally stricter
than treating JSON Schema's \`format\` as annotation-only.

This is not a general-purpose implementation of every JSON Schema 2020-12
feature. The committed schemas remain the authoritative record contracts, and
the validator's supported vocabulary is explicit and tested. Structural
validation does not adjudicate scientific truth, measurement validity, rights
clearance, regulatory compliance or whether two papers describe independent
physical prototypes. Those require source review, expert annotation and
task-specific independent verification.

## Run

From the repository root:

    cargo test -p wpt-empirical-validator --locked
    cargo run -p wpt-empirical-validator --locked

An optional first argument supplies an alternate repository root:

    cargo run -p wpt-empirical-validator --locked -- /path/to/symtropy

A successful run prints source, prototype, measurement and engineering-metric
counts. A malformed record, unsupported schema keyword, unresolved source ID,
duplicate identity, invalid qualifier or range inconsistency exits non-zero.

## Validation limits

The validator checks the declared record structure and consistency rules. It
does not prove the authors' interpretation is correct, that a measurement is
independently reproduced, that an efficiency boundary is physically
comparable, or that a reported safety result establishes legal compliance.
Those require source review, expert annotation and task-specific independent
verification.
