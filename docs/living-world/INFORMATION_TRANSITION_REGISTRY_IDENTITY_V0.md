# Information Transition Registry Identity V0

Status: normative Living World authority contract

## Purpose

Canonical reachability decisions must bind the exact transition-policy corpus that produced them. A semantic transition-registry key/version names an intended lineage; it does not prove that two registries contain the same source/destination edges, provenance rules, discard declarations, or representation bindings.

This contract is the transition-graph analogue of `INFORMATION_POLICY_REGISTRY_CONTENT_IDENTITY_V0`.

## Core invariant

**A prepared promotion/collapse/fidelity plan is valid only under the exact transition-policy graph whose canonical manifest produced that plan. Same semantic key plus different graph contents is different authority state.**

## Required authority identity

A mature transition registry SHOULD expose a stamp equivalent to:

- semantic transition-registry key/version;
- canonical manifest/profile version;
- exact transition-graph content fingerprint;
- exact bound information-policy corpus identity;
- optional execution/evidence profile identity where transition semantics depend on qualified implementation artifacts.

The concrete digest algorithm may evolve behind an explicit manifest/profile version. Canonical ordering and canonical field encoding are mandatory.

## Manifest coverage

The canonical transition manifest MUST bind every fact capable of changing legal reachability, including at least:

- transition semantic key/version;
- source representation key/schema;
- destination representation key/schema;
- every introduced information/evidence claim;
- exact R0 retained-authority descriptor;
- exact R1 lossless-transform descriptor;
- exact R2 closure evidence lineage/model/domain binding;
- exact R3 measurement-authority descriptor;
- exact R4 conditional-derivation model descriptor;
- every declared discarded-information category;
- transition-registry semantic key/version;
- manifest/profile version;
- exact information-policy corpus stamp once available.

Builder insertion order, map order, thread schedule, camera state, hardware state, and renderer state MUST NOT alter the canonical manifest or fingerprint.

## Plan binding

A prepared `InformationTransitionPlan` or reviewed successor MUST ultimately bind the exact transition authority stamp, not merely a numeric/semantic registry version.

Before canonical COMMIT, replay, save-resume, cache reuse, or federated handoff, prove that:

1. the semantic transition registry matches;
2. the exact transition graph content fingerprint matches;
3. the bound information-policy corpus identity matches;
4. every referenced edge still has identical source/destination/provenance/discard semantics;
5. any separately qualified transform/closure/measurement authority named by the path remains valid.

Any mismatch invalidates the old plan and requires reprepare.

## Replay theorem

Let `G1` and `G2` share the same semantic registry key. If any authority-relevant manifest field differs, then `stamp(G1) != stamp(G2)` and a plan prepared under `G1` MUST NOT commit under `G2`.

Human-readable version labels are never sufficient proof of this theorem.

## Save/reload

Canonical save state that contains or resumes prepared transition/fidelity work MUST preserve enough information to reconstruct/validate the same transition authority stamp.

Reload under a different graph with the same semantic key MUST fail closed rather than silently reinterpret the old plan.

## Observatory evidence

Living World Observatory evidence SHOULD record:

- transition semantic key/version;
- manifest/profile version;
- exact transition content fingerprint;
- bound information-policy corpus fingerprint;
- exact ordered path selected;
- external R0/R1/R3 prerequisites;
- R2/R4 evidence/model identities;
- cumulative discarded information.

This makes reachability decisions reproducible and auditable rather than conventional.

## Required fixtures

1. identical graph registered in different insertion order -> identical manifest/fingerprint;
2. add/remove one edge -> fingerprint changes;
3. change one source or destination schema -> fingerprint changes;
4. change one R0/R1/R2/R3/R4 provenance descriptor -> fingerprint changes;
5. change one discard declaration -> fingerprint changes;
6. same semantic registry key + different graph -> old prepared plan rejects;
7. same graph + different information-policy corpus -> authority stamp differs/rejects;
8. save/reload canonical graph reconstruction -> same stamp;
9. renderer/hardware/runtime pressure -> no effect on stamp.

## Relationship to other contracts

- `INFORMATION_POLICY_REGISTRY_CONTENT_IDENTITY_V0` binds the exact information-policy corpus.
- `INFORMATION_PROMOTION_PROVENANCE_V0` says where information may come from.
- `LOSSLESS_TRANSFORM_QUALIFICATION_V0` governs R1 implementation evidence.
- `PROCESS_ACTIVATION_ATOMICITY_V0` requires plans to remain current through COMMIT.
- `COMPOSITE_AUTHORITY_CONTEXT_V0` binds source snapshots/revisions.

## Non-goals

This contract does not require a distributed consensus protocol, prescribe one cryptographic algorithm forever, or make a transition graph sufficient by itself to authenticate external retained/measurement authority.
