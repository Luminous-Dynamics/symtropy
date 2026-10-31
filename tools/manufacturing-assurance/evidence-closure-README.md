# Transitive evidence dependency closure (v0.1)

A dependency-free Rust reference that evaluates whether a root manufacturing evidence artifact has a complete, current, scope-compatible and tier-appropriate provenance closure.

It operates on a **pre-verified receipt catalogue**. The `ACCEPTED` admission field is an input from the separate cryptographic evidence-receipt verifier; this tool does not authenticate signatures, verify raw file bytes, or calculate SHA-256 values from artifacts.

## Build and run

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/evidence_closure.rs -o /tmp/mfg-evidence-closure-tests
/tmp/mfg-evidence-closure-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/evidence_closure.rs -o /tmp/mfg-evidence-closure
/tmp/mfg-evidence-closure tools/manufacturing-assurance/fixtures/valid-evidence-closure.tsv
```

Or run the full manufacturing-assurance suite:

```sh
bash tools/manufacturing-assurance/run-tests.sh
```

## Manifest format

Pipe-delimited UTF-8; literal pipe characters are not allowed in fields.

- `CASE|id|root_artifact_id|scope|assessment_utc|minimum_evidence_tier`
- `ARTIFACT|id|role|sha256|scope|valid_from|valid_until|tier|admission_status|verified_receipt_digest`
- `DEP|consumer_artifact_id|material_artifact_id|expected_material_sha256`
- `ROLE_POLICY|consumer_role|required_material_role|minimum_tier|minimum_count`

Tiers T0–T4 are project evidence tiers and are not standards/regulatory grades. Validity uses canonical UTC and half-open intervals `[from, until)`.

## Closure policy

For each root-reachable artifact, the evaluator checks:
- exact dependency digest equality against the declared material digest;
- material role declared for the consumer role;
- required role multiplicity (`minimum_count`) and per-role minimum tier;
- artifact admission status `ACCEPTED`;
- current validity at CASE assessment time;
- scope compatibility at every dependency edge;
- no dependency cycles;
- no orphan/unreachable artifacts in the submitted package.

Scope policy:
- GLOBAL material may support a local consumer;
- local material may support only the same local scope;
- local material cannot support a GLOBAL consumer.

An absent dependency edge is not independence. It is a missing dependency whenever role policy requires it. A reference standard or calibration certificate must itself close over any required source evidence described by its own role policy.

The report includes a deterministic root-to-artifact witness path and all discovered reason codes. A closure failure is not repaired by another path: every submitted artifact is evaluated and all required role policies must be satisfied.

## Frozen fixture and tests

The valid fixture is a synthetic measurement chain:

`measurement → raw-data, calibration, instrument`

and:

`calibration → calibration-certificate, reference-standard`.

The Rust test panel includes missing direct/transitive calibration links, digest mismatch, stale or non-accepted evidence, cross-scope material, GLOBAL/local rules, insufficient tier, time boundary, role mismatch, dependency cycle, orphan artifact, unknown references, duplicate IDs/edges, malformed digests, deterministic replay, shared dependency paths, optional policy roles, and a 10,000-artifact chain.

## Complexity and scaling

The reference builds dependency adjacency once and traverses reachable nodes using breadth-first search. Cycle detection uses Kahn's algorithm. Per-artifact dependency checks operate on indexed adjacency rather than scanning the complete edge list for every artifact.

The 10,000-node test is a correctness stress test. It is not a performance claim. Measure wall time and peak memory on pinned hardware/toolchains before publishing throughput, and then extend to 100,000 and 1,000,000 nodes.

## Integration with evidence receipts and assurance cases

The intended flow is:

1. Cosign/Sigstore verifies the exact root artifact bytes and in-toto subject.
2. A maintained DSSE/in-toto parser validates the authenticated predicate.
3. The evidence-receipt verifier authenticates each material artifact/receipt and produces an ACCEPTED receipt record.
4. This closure checker validates transitive role/digest/scope/time/tier dependency closure.
5. #3178 consumes the closure report digest as evidence for the relevant claim.
6. #3165 change impact invalidates downstream closure reports when an upstream digest, calibration, firmware, model or material-lot dependency changes.

An artifact receipt digest here is a reference to upstream verifier output. The closure checker checks its syntax and admission state field only; it does not verify the receipt itself.

## Claim ceiling

This reference establishes dependency closure over declared records and pre-verified receipt statuses. It does not hash raw files, authenticate signatures, verify TUF trust-root freshness, prove measurement traceability, establish semantic relevance, or prove physical truth.
