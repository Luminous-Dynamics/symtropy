# Living World Transition Applicability Policy Identity V0

Status: normative design contract; does not by itself claim executable qualification.

## Purpose

A semantic applicability-policy key/version does not uniquely identify the authority semantics of a transition graph. Two policies may reuse the same human version while assigning different transitions to `Universal` or `RegisteredDomain(D)`.

This contract requires canonical plans, replay, persistence, caches, Observatory evidence, and future federation to bind the exact applicability-policy corpus.

## Core theorem

**Canonical transition applicability binds exact policy content. Any change to whether an edge is Universal or domain-gated, or to the domain descriptor attached to an edge, changes applicability authority identity even when the semantic policy key/version is reused.**

## Authority stamp

A mature authority identity contains at least:

- semantic `TransitionApplicabilityPolicyKey`;
- canonical manifest/profile version;
- exact applicability content fingerprint;
- exact binding to the transition-graph authority identity;
- optional exact binding to domain-authority registry/profile identities where the manifest version requires it.

The human key/version identifies intended lineage. The exact fingerprint identifies the corpus actually used.

## Canonical manifest

The deterministic manifest fingerprints at least:

- applicability semantic key/version;
- manifest/profile version;
- exact transition-graph authority binding;
- every transition key in canonical order;
- exact applicability mode for every transition;
- for `RegisteredDomain`, exact `TransitionDomainKey`;
- any future applicability metadata capable of changing canonical admissibility.

Registration order, map order, thread scheduling, or host behavior cannot affect the manifest.

## Authority-significant changes

All of the following change exact applicability identity:

- `Universal -> RegisteredDomain(D)`;
- `RegisteredDomain(D) -> Universal`;
- `RegisteredDomain(D1) -> RegisteredDomain(D2)`;
- adding or removing an edge entry;
- rebinding the same applicability map to a different transition-graph corpus;
- changing manifest semantics/version incompatibly.

In particular, removing a domain requirement from a later policy cannot retroactively make an old prepared plan commit-valid.

## Plan and proof binding

An applicable transition/candidate plan must ultimately carry the exact applicability authority stamp, not only the semantic key.

A per-state domain proof is evaluated under one exact applicability mapping. If the mapping changes after PREPARE, that proof cannot be reused automatically even when the underlying domain authority remains qualified.

COMMIT validates:

- exact applicability authority identity;
- exact transition-graph identity;
- required domain descriptors still match;
- bound per-state domain proofs correspond to those descriptors;
- relevant authority status remains current.

Any mismatch yields stale/refuse with zero ecological mutation.

## Candidate-path selection

Every candidate evaluated under the path-discovery/selection contract must be annotated using one exact applicability-policy corpus. Candidate A evaluated under applicability corpus X and candidate B evaluated under corpus Y cannot enter one canonical selection decision as though their admissibility proofs were comparable.

The final selected plan binds the exact applicability corpus used during filtering and selection.

## Replay and persistence

Save/reload and replay reconstruct the same applicability authority identity from the same canonical manifest. A semantic policy label alone is insufficient to resume prepared work.

Living World Observatory evidence records the exact applicability stamp alongside information-policy, transition-graph, domain, transform, closure, and source-state identities relevant to the decision.

## Qualification fixtures

A compliant implementation should demonstrate:

- identical policy registered in reverse order -> identical fingerprint;
- Universal -> RegisteredDomain changes fingerprint;
- D1 -> D2 changes fingerprint;
- add/remove entry changes fingerprint;
- same semantic key + different content rejects an old plan;
- same applicability mapping bound to a different transition graph changes authority;
- save/reload reconstructs identical stamp;
- renderer/camera/FPS/hardware state does not affect fingerprint;
- applicability corpus changes after PREPARE -> COMMIT rejects with zero mutation.

## Non-goals

This contract does not define distributed consensus, does not make a semantic version a credential, and does not itself prove that any particular domain evaluator is qualified.