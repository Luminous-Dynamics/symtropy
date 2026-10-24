# Living World Transition Candidate Admissibility V0

Status: normative design contract; does not by itself claim executable qualification.

## Purpose

Bounded transition discovery and applicability annotation still do not prove that a candidate may participate in canonical fidelity selection. A candidate can fail because of missing information, stale source authority, unresolved state domains, unqualified transforms, unavailable measurement/retained authority, closure mismatch, or spatiotemporal invalidity.

This contract prevents canonical selection from accepting caller-authored `admissible=true` assertions or arbitrary scores.

## Core theorem

**Canonical fidelity selection consumes only authority-derived candidate admissibility certificates produced under one coherent decision context. Structural existence, array position, and caller assertions are not admissibility evidence.**

## Candidate identity

A certificate identifies the semantic candidate independently from enumeration position. Mature identity binds at least:

- exact information-policy corpus authority;
- exact transition-graph authority;
- exact applicability-policy authority;
- source/destination representation identities;
- exact ordered transition sequence;
- cumulative information-loss/provenance semantics;
- candidate-search/candidate-set identity where relevant.

Changing any authority-relevant corpus changes candidate identity even when the ordered transition keys happen to look the same.

## Coherent decision context

Candidates compared in one canonical decision share exactly one context:

- authority scope;
- source snapshot/revision/non-reused state identity;
- composite authority context/source revisions;
- process-set generation and registered requirements;
- information-policy corpus;
- transition-graph corpus;
- applicability-policy corpus;
- spatiotemporal evaluation context;
- closure/evidence corpus;
- semantic selection-policy generation/content identity.

Certificates from different contexts cannot be mixed.

## Admissibility evidence

A candidate certificate binds all evidence needed to establish admissibility, including where relevant:

- process-information sufficiency;
- spatiotemporal sufficiency;
- coherent composite-source validation;
- applicability requirements and exact per-state domain proofs;
- R0 retained-authority resolution;
- R1 qualified lossless-transform authority;
- R2 typed closure acceptance/evidence;
- R3 measurement authority/evidence;
- R4 conditional-state constraints;
- source/evidence freshness and revocation state.

The canonical authority resolver owns certificate construction. A public free constructor that allows arbitrary callers to mint an admitted certificate is forbidden.

## Rejection is typed evidence

Rejected candidates remain useful Observatory evidence. Deterministic rejection reasons should distinguish at least:

- insufficient information;
- spatial/temporal mismatch;
- stale source or composite source;
- unresolved/unsatisfied domain;
- unqualified/revoked R1 transform;
- unavailable R0/R3 authority;
- closure observable/metric/horizon mismatch;
- corpus identity mismatch;
- incomplete candidate discovery.

Rejection performs zero ecological mutation.

## Staleness

Admissibility is not permanent. Any authority input relied on by a certificate changing before SELECT/COMMIT makes the certificate stale unless a higher-level epoch/linearization guarantee proves immutability over the transaction.

Examples include source revision, process generation, policy corpus, domain/transform/closure/measurement status, and composite-source revision.

## Selector boundary

The canonical selector consumes only current admitted certificates from one coherent decision context.

It must not accept:

- raw `TransitionCandidatePath` as proof of validity;
- `bool admissible`;
- arbitrary caller-authored numeric scores;
- certificates from different snapshots/policy corpora;
- stale or revoked certificates.

Selection then applies an explicit versioned semantic policy to the admitted set.

## Qualification fixtures

A compliant implementation should demonstrate:

- one missing domain proof rejects the candidate;
- forged caller `admitted=true` cannot enter selector API;
- R1 authority revoked after certification makes the certificate stale;
- source revision/state token change invalidates dependent certificates;
- same transition sequence under different policy corpora has different candidate authority identity;
- certificates from different snapshots cannot be combined;
- rejected shortest path + admitted longer path leaves longer path selectable;
- no admitted candidate produces explicit refuse/defer with zero mutation;
- record/thread ordering does not alter certification for equal canonical input;
- renderer/camera/FPS/hardware state cannot affect admissibility.

## Non-goals

This contract does not rank candidates, does not permit performance-driven semantic choices, and does not imply that rejected candidates must be permanently retained after their evidence has been recorded.