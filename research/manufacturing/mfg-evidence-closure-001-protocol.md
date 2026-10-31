# MFG-EVIDENCE-CLOSURE-001 — Transitive, digest-bound evidence dependency closure

**Version:** 0.1  
**Date:** 2026-10-31  
**Evidence tier:** T0 synthetic structural  
**Implementation:** `tools/manufacturing-assurance/evidence_closure.rs`

## 1. Research question

Can a manufacturing digital-twin evidence item be admitted only when its complete transitive provenance is present, digest-bound, role-compatible, current, in-scope, and supported at the required evidence tier?

A valid signature on a root report does not make the report decision-grade if its underlying calibration, raw measurement, reference standard, model, firmware, material-lot, or configuration dependencies are absent or stale.

## 2. Why top-level attestation is not enough

Examples of incomplete evidence:
- a process-quality measurement references raw sensor data but omits the calibration certificate;
- the calibration record references a standard that is expired or belongs to another instrument/site;
- a model result is signed but was generated with a different firmware/model/configuration digest than the manifest declares;
- a material-lot certificate has a familiar filename but a different exact digest;
- an intermediate receipt is accepted but does not close over its own dependencies.

A cryptographically authentic statement can make a false or incomplete statement attributable. It does not make its claims true. Dependency closure is therefore a separate gate after signature/identity verification and before assurance-case admission.

## 3. Paradigm shift: dependency-closed evidence packages

Make the complete evidence package the unit of admission. Each edge declares the exact consumer artifact, material artifact, and expected digest. Each consumer role has an explicit policy for required material roles, multiplicity, and minimum evidence tier.

A filename, URL or producer assertion is not an artifact identity. The expected digest is compared to the digest field of an artifact record, which must itself be bound by an upstream verified receipt. Missing edges are not treated as independence. Unreachable/orphan artifacts are reported rather than silently tolerated.

This is a structural closure prototype; the actual hashing and cryptographic verification remain the responsibility of the Cosign/Sigstore/in-toto receipt verifier.

## 4. Manifest types

### CASE

`CASE|id|root_artifact_id|scope|assessment_utc|minimum_evidence_tier`

The case chooses a root, scope, assessment time and minimum evidence tier.

### ARTIFACT

`ARTIFACT|id|role|sha256|scope|valid_from|valid_until|tier|admission_status|verified_receipt_digest`

The artifact digest identifies exact bytes; the receipt digest identifies upstream verifier output. The tool only syntax-checks both digests and accepts an artifact only if the supplied admission state is `ACCEPTED`. This state is assumed to have been emitted by a separate trusted verifier and must not be user-controlled in production.

### DEP

`DEP|consumer_artifact_id|material_artifact_id|expected_material_sha256`

Every edge binds an exact dependency digest. Duplicate, unknown, self, mismatched-digest and cyclic dependencies are rejected or block closure.

### ROLE_POLICY

`ROLE_POLICY|consumer_role|required_material_role|minimum_tier|minimum_count`

The policy declares the required roles per consumer type. A role with minimum count zero is optional; an undeclared role edge is rejected as an unexpected dependency rather than silently accepted.

## 5. Evaluation semantics

For each root-reachable artifact:
- its own admission state must be ACCEPTED;
- its validity interval must contain the CASE assessment time;
- its scope must cover the CASE scope;
- its role policies must have the required number of dependency edges;
- each edge's expected digest must match the material artifact digest;
- each dependency role must be declared by the consumer's role policy;
- material evidence tier must meet that role's minimum tier;
- material scope must be GLOBAL or exactly match the consumer scope.

The graph must be acyclic. Every artifact supplied in the package must be root-reachable; orphan artifacts are reported, not ignored. The report returns deterministic root-to-artifact witness paths and reason codes.

All validity intervals are half-open `[valid_from, valid_until)`. A material is current at the assessment time only when `valid_from <= as_of < valid_until`.

## 6. Frozen known-truth fixture

The valid synthetic measurement case contains six artifacts:
- process-measurement output;
- raw measurement data;
- calibration record;
- instrument identity;
- calibration certificate;
- reference standard.

The root measurement requires raw data, calibration and instrument identity. The calibration record itself requires a calibration certificate and a reference standard. All materials are digest-bound, T2, ACCEPTED and valid at the synthetic assessment time.

Fault mutations cover missing direct calibration, missing transitive certificate, mismatched digest, stale calibration, non-accepted receipt, wrong scope, local/global scope error, root/material tier mismatch, validity boundary, role confusion, cycles, orphan artifacts, unknown references, duplicate artifact/dependency IDs and malformed SHA-256.

## 7. Benchmark goals

These are proposed acceptance targets, not measured results.

| Axis | T0 target | Advanced target |
|---|---|---|
| Dependency completeness | reject every frozen missing-role case | generated graph mutation campaign and independent checker |
| Exact binding | 100% reject digest mismatch | verify real artifact bytes and authenticated receipts |
| Time | exact half-open boundary behavior | trusted time and clock uncertainty |
| Scope | exact local/global edge rules | nested scopes and multi-tenant sites/regions |
| Evidence tier | enforce role minimum tier | domain-specific evidence profiles and claim-specific thresholds |
| Graph integrity | reject cycles, unknown refs, duplicates, orphans | adversarially large DAGs and graph mutation |
| Explainability | deterministic root-to-node witness and reason codes | minimal missing-edge cut set and reviewer localization |
| Scale | correctness at 10k artifacts | measure runtime/memory at 100k and 1M nodes |
| Provenance integrity | consume ACCEPTED receipt status only | verify bytes, signatures, identity, TUF root and revocation in a separate verifier |

False closure is the dangerous error. Report false closure, false rejection, missing-role localization, count of dependency edges and package size separately. Never compress these into one score that can hide a missing calibration edge.

## 8. Complexity and scale protocol

The implementation builds adjacency once, uses BFS for root-reachable witness paths and Kahn's algorithm for cycle detection. Dependency records are indexed by consumer so per-node checking scales with local degree rather than scanning the full edge list for every node.

The 10,000-artifact test validates correctness under a long chain. It does not claim a runtime or memory benchmark. Performance claims require:
- exact compiler/toolchain and optimization profile;
- pinned hardware/OS;
- repeated runs and p50/p95/p99;
- peak resident memory;
- graph shape distribution (chain, star, layered DAG, shared dependencies, high fan-in/out);
- separate parsing and evaluation times.

## 9. Integration

- #3184 cryptographic receipt verification binds root/material artifact bytes to trusted signer identity and current trust root.
- This closure checker consumes the resulting verifier receipt catalogue and verifies dependency completeness.
- #3178 consumes the closure report as evidence for a claim.
- #3165 change impact invalidates downstream closure results if any dependency digest, calibration, firmware, model, configuration or material lot changes.
- #3180 intervention plans must reference a closed assurance evidence package before their offline transaction record can be admitted.

The exact closure report and package digest must be recorded. Branch names, filenames and mutable PR URLs are not sufficient evidence identity.

## 10. Claim ceiling

This prototype validates closure over declared artifact records and admission statuses. It does not hash raw file bytes, verify signatures or TUF metadata, prove metrology traceability, establish semantic relevance, or prove model/physical validity. The ACCEPTED flag is assumed to come from a separate trusted cryptographic verifier; this tool must not be treated as the cryptographic verifier itself.
