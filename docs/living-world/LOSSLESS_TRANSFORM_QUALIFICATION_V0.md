# Lossless Transform Qualification V0

Status: normative Living World authority contract

## Purpose

R1 `LosslessDerivation` is the only promotion-provenance class that claims new exact canonical information can be derived from currently retained canonical state without another retained owner or a new measurement. That claim is strong enough that a semantic transform name/version cannot serve as proof by itself.

## Core invariant

**An R1 transition may introduce Exact canonical information only when the exact transform implementation/profile has independent evidence that the destination facts follow losslessly and deterministically from the admitted source state. Determinism alone is not losslessness.**

A transition graph may name a transform descriptor. Canonical COMMIT must resolve that descriptor to qualified transform authority.

## Qualified transform identity

A qualified R1 record SHOULD bind at least:

- semantic `LosslessTransformKey`;
- exact implementation/source/artifact fingerprint;
- source representation and schema/version;
- destination representation and schema/version;
- exact information claims introduced by the transform;
- admitted source-state domain/profile;
- deterministic transform algorithm/profile version;
- evidence lineage / qualification capsule identity;
- current qualification status: qualified, revoked, or superseded;
- where relevant, compiler/toolchain/runtime/capsule identity capable of changing executable behavior.

Reusing the semantic transform key for a different implementation is not the same authority record.

## Required proof obligations

Qualification MUST establish evidence appropriate to the transform that covers at least:

1. identical canonical input produces identical canonical output;
2. output is independent of thread ordering, map insertion ordering, camera state, frame rate, wall clock, GPU scheduling, and other non-canonical inputs;
3. every introduced Exact fact is logically/mathematically implied by source canonical state over the admitted domain;
4. source schema/version exactly matches the qualified domain;
5. destination schema/version exactly matches the qualified codomain;
6. extensive quantities, counts, ownership coordinates, or identity facts covered by the transform are preserved exactly according to the transform contract;
7. truncation, overflow, rounding, sentinel, null, and boundary cases fail closed or are proven exact over the admitted domain;
8. a bijection/inverse/roundtrip proof exists when the transform claims reversible encoding, or another reviewed proof establishes injectivity/information preservation;
9. save/reload/replay reproduces identical transformed authority;
10. implementation/profile/evidence drift invalidates the old qualification.

## R1 versus other provenance classes

Legitimate R1 examples MAY include:

- exact canonical schema/layout conversion with a proven bijection;
- exact fixed-point unit conversion over a bounded domain where forward/inverse checked arithmetic is proven lossless;
- deterministic canonical normalization with a proven inverse/unique decoding;
- equivalent sparse-index/storage representations preserving the complete occupied state.

The following are NOT R1 merely because they are deterministic:

- sampling individuals from marginals;
- reconstructing covariance from moments/marginals;
- interpolation/extrapolation of missing history;
- pseudorandom or hashed microstate filling;
- ML inference;
- lossy binning/coarsening;
- approximating local contact or continuous state from coarse fields.

Those must remain R2 closure evidence, R4 conditional D-state, measurement, or another explicitly qualified evidence class.

## Registry ownership

Canonical runtime SHOULD resolve R1 transforms through a sealed qualified-transform registry or reviewed equivalent.

Callers may request/use a semantic transform descriptor. They MUST NOT provide their own `transform is lossless = true` assertion as canonical evidence.

A prepared transition plan binds the resolved transform authority stamp. If the transform is revoked, superseded, recompiled into a materially different qualified artifact, or mismatched to source/destination schema after PREPARE, the plan is stale and COMMIT MUST fail with zero ecological mutation.

## Composition

A multi-edge transition path may contain more than one R1 transform. Every R1 edge must resolve independently, and the prepared plan must bind every transform authority it depends on.

Qualification of `A -> B` and `B -> C` does not automatically prove an undeclared direct `A -> C` transform; canonical planning may compose the two registered edges while preserving both evidence identities.

## Interaction with information loss

An R1 transform MUST NOT be used to hide a prior information-losing collapse. If exact information was discarded and no retained exact owner remains, a later deterministic function over the coarse state cannot be certified as an R1 restoration of that forgotten history.

This is true even when the function produces values in the destination schema and is perfectly deterministic.

## Required fixtures

1. exact bijective fixture qualifies;
2. same source records under different insertion/thread order -> identical canonical output;
3. roundtrip boundary fixtures preserve every canonical bit/unit admitted by the profile;
4. lossy binning rejects R1 qualification;
5. deterministic pseudo-strata reconstruction rejects R1 qualification;
6. source schema mismatch rejects;
7. destination schema mismatch rejects;
8. semantic key reused with a different implementation fingerprint does not validate an old plan;
9. transform revoked after PREPARE -> COMMIT rejects with zero mutation;
10. save/reload resolves the same transform authority stamp and output;
11. Observatory evidence can explain which R1 qualification authorized every introduced Exact claim.

## Relationship to other contracts

- `INFORMATION_PROMOTION_PROVENANCE_V0` defines R0-R4 semantics.
- `INFORMATION_TRANSITION_REGISTRY_IDENTITY_V0` binds the exact transition graph that references the transform.
- `INFORMATION_POLICY_REGISTRY_CONTENT_IDENTITY_V0` binds the exact representation/process policy corpus.
- `PROCESS_ACTIVATION_ATOMICITY_V0` makes revocation/profile drift a stale-plan condition.

## Non-goals

This contract does not promise automatic proof of arbitrary programs, require every fidelity transition to have an R1 path, or permit approximate reconstruction to be relabeled exact because it is repeatable.
