# Living World Transition Domain Authority V0

Status: normative design contract; does not by itself claim Rust implementation or executable qualification.

## Purpose

Structural transition reachability is not enough to authorize a state-conditioned fidelity transition. A transition may be registered between two schemas and still be valid only for a strict subset of source states.

This contract freezes the authority boundary for proving that one exact current canonical source state lies inside one qualified transition applicability domain.

## Core theorem

**A state-conditioned transition may proceed only when a qualified domain evaluator/authority proves that the exact canonical source state, at the exact authority scope, snapshot, revision, and non-reused state identity used by the prepared transition, satisfies the exact domain profile bound to that transition.**

A semantic domain descriptor is not a proof, and a caller-provided boolean is never canonical authority.

## Three identities must remain distinct

### 1. Domain descriptor

`TransitionDomainKey`

Names the semantic theorem/profile family and version, for example a theorem describing when marginals uniquely determine a joint state.

This is a descriptor only.

### 2. Qualified domain authority

A domain authority record binds at least:

- domain descriptor;
- admitted source representation/schema;
- predicate/profile manifest version;
- exact evaluator/profile implementation or content fingerprint;
- qualification evidence lineage;
- status such as qualified, revoked, or superseded;
- exact admitted applicability domain.

A semantic domain key reused with a different implementation/content fingerprint denotes different authority.

### 3. Per-state domain proof

A successful proof binds at least:

- exact qualified domain authority;
- canonical authority scope;
- source representation/schema;
- canonical snapshot identity;
- source revision/generation;
- opaque non-reused source-state token/fingerprint;
- deterministic evaluation result;
- evaluator/profile version and evidence reference where applicable.

A proof is valid only for the exact evaluated state identity.

## Fail-closed rules

1. Canonical code must not accept `domain_satisfied: bool` as proof.
2. Unknown, revoked, or superseded domain authorities fail closed.
3. Domain authority must be bound to the transition's exact source representation/schema.
4. Per-state proof must bind exact scope, snapshot, revision, and source-state identity.
5. Source mutation after evaluation makes the proof stale.
6. Rollback or alternate lineage under a reused numeric revision must still fail through the non-reused state token/fingerprint.
7. Domain implementation/content drift invalidates old proof authority even when the semantic domain key is unchanged.
8. Domain evaluation may use only canonical inputs admitted by its qualified profile; camera, renderer state, FPS, wall clock, GPU scheduling, thread order, and ECS iteration order are invalid authority inputs.
9. Equal canonical input under equal qualified authority must produce equal result independent of input record ordering.
10. Unsatisfied, missing, stale, unknown, or revoked domain evidence causes zero ecological mutation.

## State-dependent reachability example

Generic independent marginals do not determine exact covariance:

`age marginal + condition marginal -/> exact age x condition joint state`.

But an explicitly qualified theorem may establish unique reconstruction for a strict domain such as one occupied age bin and one occupied condition bin.

In that case:

`registered transition edge`
`+ qualified R1 transform`
`+ qualified domain authority`
`+ exact current source state satisfies domain`
`-> transition may become commit-eligible`.

If a second condition bin appears after PREPARE, the old domain proof is stale.

Deterministic pseudo-sampling is not a domain theorem and cannot be used to make a generic lossy source exact.

## Relationship to R1 transform qualification

Lossless-transform qualification and state-domain applicability answer different questions.

R1 transform qualification proves that a particular exact transform implementation is deterministic and lossless over an admitted domain.

Transition-domain authority proves that the current canonical source state is inside that admitted domain.

Both are required for a state-conditioned R1 exact promotion. Neither may silently widen the other's admitted domain.

## Prepared-plan binding

Any prepared realization, promotion, collapse, or process-activation plan that relies on a state-conditioned edge must bind:

- exact information-policy corpus authority;
- exact transition-policy graph authority;
- exact applicability-policy authority;
- every required domain authority stamp;
- every per-state domain evaluation subject/proof;
- relevant source revisions/state identities;
- required R0/R1/R3 external authority evidence.

COMMIT revalidates those bindings. Any mismatch returns stale/refuse with zero mutation.

## Replay and persistence

Save/reload, cache reuse, evidence replay, or future federation must not reconstruct a domain decision from only a human version label. The exact domain authority/content identity and evaluated source-state identity must remain inspectable.

## Qualification fixtures

A compliant implementation should demonstrate at least:

- qualified degenerate-state theorem + matching source revision succeeds;
- generic multi-bin marginals fail the same theorem;
- same semantic domain key with changed evaluator fingerprint rejects old proof;
- source revision changes after proof -> stale;
- source state token changes under same revision -> stale;
- revoked domain authority after PREPARE -> COMMIT rejects;
- canonical record-order permutations yield the same result;
- renderer/camera/FPS changes have no effect;
- caller-forged boolean/proof cannot enter canonical commit API.

## Non-goals

This contract does not require every transition to be state-conditioned, does not permit arbitrary user scripts in `lifesim-core`, and does not attempt to automatically prove arbitrary predicates correct.