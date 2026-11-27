# AERO Portable Assurance Capsule v0.1

**Status:** research format / structural contract  
**Branch:** `research/aero-assurance-capsule-v0.1`  
**Standards relationship:** complementary to ISO/IEC/IEEE 15026-2:2022 assurance-case structure  
**Claim ceiling:** portable assurance-case report and replay instructions; no finding of argument soundness, physical truth, airworthiness, engineering acceptance or certification

## 1. Purpose

AERO should be able to hand a reviewer a portable, versioned, independently reconstructable assurance-case report, not merely a dashboard or a link to a mutable document collection.

The capsule contains three distinct things:

1. **The assurance argument:** the proposition, decomposition strategy, contexts, assumptions, evidence claims, defeaters, rebuttals and judgements.
2. **The evidence index:** the exact content digests, owner record identities, provenance roles, availability/disclosure state and receiver-first-seen intervals.
3. **The reproduction envelope:** verifier source/build/dependencies, exact inputs, invocation, runtime/network policy, output commitments and unresolved prerequisites.

These objects must not be collapsed. An argument graph is not itself evidence. A digest is not owner authentication. A reproducible computation does not make a physical premise true.

## 2. Standards position

ISO/IEC/IEEE 15026-2:2022 specifies assurance-case structure and terminology so cases can be developed, maintained and communicated consistently. It does **not** set requirements for the quality or soundness of the case's contents and does not prescribe a particular graphical notation or physical implementation.

AERO should align its report structure and vocabulary with that standard while providing a stronger computational envelope around:
- typed claim and context identity;
- exact evidence and owner receipt references;
- non-compensatory component dispositions;
- temporal knowledge and trust snapshots;
- model and metric semantic bindings;
- verifier/build identity;
- detached package integrity receipts;
- independent reconstruction instructions.

The capsule is a machine-readable report/index and evidence package contract, not a new assurance-case standard. A competent engineer and the appropriate authority must still determine whether the argument is adequate and sound.

## 3. Capsule structure

### A. Claim context

The capsule binds the claim ID, physical subject, configuration, operating envelope, intended use, specification raw-payload digest, typed semantic digest, decision-context digest, acquisition interval, historical as-known cutoff, trust-policy digest and model-applicability profile.

The raw specification digest, typed semantic commitment and decision-context digest are different commitments. They cannot be substituted for each other.

### B. Argument graph

The graph supports typed nodes:
- claim;
- strategy;
- context;
- assumption;
- evidence;
- defeater;
- rebuttal;
- judgement.

Typed edges include decomposition, support, context, assumption, evidence, attack, rebuttal and qualification. Every node has an explicit status, reason-code set and limitations. Nodes and edges have stable IDs.

The graph validator must reject duplicate node IDs, dangling edges, cycles in the argument graph, missing root claims and evidence nodes without exact evidence-manifest entries. Unreachable/orphan nodes are incomplete rather than silently ignored. Argument labels and edge types do not prove logical validity; independent reviewers must evaluate the inference.

### C. Evidence manifest

Every evidence item is bound to:
- exact evidence identity and owner namespace;
- semantic role;
- raw payload and owner-record SHA-256;
- byte length and media type;
- availability (included, digest-only, withheld, authorized access required, unavailable);
- receiver-first-seen interval;
- owner receipt identity and exact receipt digest;
- whether the item is mandatory to the claim.

The evidence index must preserve selective disclosure. If a source is restricted or withheld, the capsule must say so and must not call itself fully reproducible. A public digest is not a substitute for the actual evidence bytes or owner verification.

### D. Component results and decision

Component results preserve each owner’s own disposition and reason codes. They are not combined with weights or majority vote. A positive top-level decision cannot compensate for a rejected, incomplete, out-of-scope, unverified or late mandatory component.

The top-level AERO disposition is at most `REVIEWABLE_FOR_INDEPENDENT_DECISION`. It is not physical approval, certification, airworthiness or an authorization to act.

### E. Verifier and reproduction envelope

The verifier manifest records source commit/tree, build artifact digest, dependency lock digest, invocation, runtime, network policy, input/output manifest commitments and execution receipt.

Reproduction classification is explicit:
- `fully_reproducible`: every required byte and dependency is included, no unresolved prerequisite remains, and the declared replay environment is pinned;
- `digest_only`: commitments are present but required bytes are not included;
- `authorized_access_required`: complete reconstruction depends on owner-authorized access;
- `incomplete`: required evidence, receipts, build identities or prerequisites are missing.

The verifier should run offline where possible. Any pinned external dependency must be enumerated and separately disclosed. A "hermetic" label requires actual sandbox/network enforcement evidence, not caller assertion.

## 4. Package integrity and detached receipts

A capsule cannot embed the hash of its own exact serialized bytes without a self-referential construction. Therefore:
- the in-capsule `canonicalization_profile` describes canonical serialization of the JSON payload under RFC 8785 JCS;
- the exact serialized capsule bytes are hashed externally;
- a detached owner receipt binds the capsule's exact payload SHA-256, byte length, signer/authority, receiver-first-seen interval and trust-policy snapshot;
- the detached receipt itself is referenced by a separate outer package manifest or trusted owner receipt index.

This keeps exact-byte identity separate from typed semantic commitments and decision-context commitments. Signature and authority verification remains with the appropriate owner-native trust boundary.

## 5. Disclosure and portability

A capsule may be public, restricted, redacted or authorized-access-only. It must distinguish:
- bytes included in the export;
- digest-only evidence;
- evidence withheld by policy;
- evidence that requires owner authorization;
- unavailable evidence.

The package must not infer that because a reviewer can see a hash, the reviewer can reproduce the engineering decision.

For long-lived infrastructure, the format should support reassembly from a content-addressed archive, with immutable owner receipts and versioned verification instructions. Changes and corrections create new capsule versions; they never rewrite the old historical record.

## 6. Relationship to AERO components

- #7781 provides the typed claim specification.
- #7805 proposes the claim-to-model metric semantic binding.
- #7742 composes model credibility into the claim gate.
- #7765 proposes owner-native model-report artifact assurance.
- Track U / #5371 owns bitemporal dependency impact and targeted requalification.
- BASALT / #7352 owns freshness, observer coverage and dependency completeness.
- #7455 owns chronology and historical replay.
- #5721 owns assurance-case adversarial challenge work.

The capsule consumes their typed results and owner receipts. It does not rebuild their trust registry, evidence database, causal observer, chronology service, VVUQ engine or attack generator.

## 7. Challenge programme

The challenge seed contains 26 proposed cases covering duplicate IDs, missing root, dangling edges, graph cycles, orphan nodes, evidence-reference errors, component blockers, reason-code loss, false reproducibility, late receiver knowledge, missing verifier identity, unqualified network dependencies, missing detached receipts, authority-ceiling violation, coupled faults and ordering metamorphisms.

The corpus remains `NOT_INDEPENDENTLY_REVIEWED` and `NOT_EXECUTED`. CI checks schema and basic cross-field graph/evidence/reproduction invariants only. Independent semantic review must include assurance-case reviewers and domain engineering reviewers. Do not derive oracle outputs from the capsule validator implementation.

## 8. Non-claims

A schema-valid capsule proves only that the exported package conforms to a declared data contract. It does not prove:
- that the claim is the correct engineering requirement;
- that the argument is logically sufficient or sound;
- that the underlying evidence is authentic or physically correct;
- that model assumptions are adequate for the context of use;
- that uncertainty is acceptable;
- that a physical asset is safe;
- that an authority has approved the decision.

Those require independent owner trust verification, competent engineering judgement, model VVUQ, physical test and the applicable approval/regulatory process.
