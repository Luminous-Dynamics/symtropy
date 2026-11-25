# ErgoStream claim-scope digest contract v0.1

**Envelope schema:** `schemas/transfer_claim_scope_envelope_v0.schema.json`  
**Registry schema:** `schemas/transfer_claim_scope_registry_v0.schema.json`  
**Reproducible fixture:** `examples/transfer_claim_scope_registry_v0.example.json`

## Purpose

A probability forecast is meaningful only when it is attached to the exact claim being scored. A claim ID alone is not enough: the same ID could be reused for another map, another episode pair, or a different set of selected states/actions. A digest alone is not enough either: the evaluator must independently reconstruct what was hashed.

The envelope binds:
- the stable claim ID and claim type;
- source and target episode IDs;
- source and target evidence-prefix SHA-256 identities;
- the complete claim artifact for that type.

The digest is:

```text
claim_scope_sha256 =
  "sha256:" + lowercase_hex(SHA-256(UTF8(JCS(claim_scope_envelope))))
```

where JCS is the JSON Canonicalization Scheme in [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html). RFC 8785 defines deterministic JSON serialization intended for hashing/signing: object properties are sorted recursively, JSON array order is preserved, and the result is UTF-8 encoded. It is an Informational RFC, not an Internet Standards Track specification. Use a tested RFC 8785 implementation in production; do not hand-roll canonicalization in the episode runner.

## Exact envelope mapping

For each forecast in AgentTransferBundle v0:

| `claim_type` | `claim_artifact` |
|---|---|
| `structural_correspondence` | the complete `structural_transfer` object |
| `deterministic_dynamics` | the complete `dynamics_transfer` object |
| `simulation` | the complete `behavioural_transfer` object whose `map_kind` is simulation |
| `bisimulation` | the complete `behavioural_transfer` object whose `map_kind` is bisimulation |
| `mechanism_transfer` | the complete `mechanism_transfer` object |

The source/target IDs and evidence-prefix digests are copied from `agent_submission_binding`, not retyped from an untrusted source. The envelope schema restricts the claim artifact to the matching typed schema.

The artifact is included in full. It therefore binds selected and excluded scopes, mappings, transformations, action/state relations, evidence IDs and frozen predictions. For mechanism claims it includes the source intervention evidence and predicted target effects, but never the actual evaluator-owned target effects or target evidence receipts.

## What is deliberately excluded

Do not hash:
- forecast probability or assert/reject/abstain decision;
- evaluator outcome labels or evaluation-stratum IDs;
- hidden target intervention outcomes or held-out transitions;
- evaluator scoring results;
- mutable timestamps, log messages or presentation-only metadata.

Those values either occur after freeze or belong to the evaluator. Including an outcome in the candidate claim digest would leak the answer and invalidate the blind experiment.

## Canonicalization rules and array ordering

- Apply RFC 8785 JCS to the entire envelope object.
- JCS sorts object property names recursively using its specified string ordering.
- JCS does **not** reorder arrays. Array order is therefore part of this exact claim identity.
- Producers must keep set-like identifier lists and mapping lists in deterministic order before freezing (ascending ID / source-target pair order). If a list order changes, the claim digest changes, even where a human might regard the two lists as semantically equivalent.
- Use only I-JSON-compatible numbers. All current entity/action/state IDs are bounded integers safely representable as IEEE-754 binary64 values.
- Do not normalize Unicode strings; the JCS contract preserves the supplied string code points. Use the same exact episode IDs and evidence IDs throughout.

## Owner verification sequence

1. Resolve the terminal AgentAction::Submit frame and owner receipts.
2. Resolve source/target public evidence prefixes by episode ID and prefix digest.
3. Extract the candidate claim artifact for the declared claim type.
4. Reconstruct the canonical envelope using owner-resolved episode IDs and prefix digests.
5. Canonicalize with an independently maintained RFC 8785 implementation.
6. SHA-256 the canonical UTF-8 bytes and compare to the candidate `claim_scope_sha256`.
7. Retain the owner-recomputed digest in the evaluator's `BoundTransferOutcomeV0`.
8. Only then call `evaluate_bound_transfer_calibration`, which requires forecast and evaluator records to agree on claim ID, type and digest before calculating scores.

**Boundary:** `evaluate_bound_transfer_calibration` validates digest syntax and exact equality against the evaluator-supplied outcome; it does not itself recompute JCS or SHA-256. The trusted owner adapter must perform steps 1–7. A matching digest is a content-identity check, not proof that observations are true, that receipts are authentic, or that the learner saw no hidden outcomes.

## Reproducible example digests

The registry example contains three claim envelopes extracted from `examples/agent_transfer_bundle_v0.example.json`:

| Claim | Type | Scope digest |
|---|---|---|
| structural-main | structural_correspondence | `sha256:671a33417589dd928c819d5f3b2ba1b9c125f96a9961797df1d71747a0c4e711` |
| behaviour-main | simulation | `sha256:6c4238994f641ce7986ddbc24a4bbba780589812cab8619eed81cf64efb27c18` |
| mechanism-main | mechanism_transfer | `sha256:9af2908a0b8ba846b15afbc4fafb88cfc4a9a4656566083b99839921e378e327` |

These are computed from the exact canonical envelopes in the example registry. Changing a scope, mapping, evidence-prefix identity, claim type, claim ID, transformation or predicted effect requires a new digest.

## Security and scientific limitations

- SHA-256 is not a signature, MAC or proof of ownership.
- The learner can propose a syntactically valid digest; only the trusted owner can establish the digest corresponds to the actual frozen artifact.
- A correct digest does not establish causal identification, intervention validity or mechanism truth.
- A correct qualitative effect-sign mapping is not a quantitative causal-effect equivalence.
- Mechanism transfer is qualified only over the declared action set and evidence receipts. Out-of-scope actions remain unknown.
- The current v0 mechanism witness checks qualitative preserve/negate effect signs. It does not estimate average treatment effects, confidence intervals, heterogeneous effects, mediators or confounding.

## Related work

- Symtropy #7173: finite structural, dynamics, behavioural and mechanism witnesses plus bound-claim calibration.
- Symtropy #7193: paired-world transfer Atlas and false-unification controls.
- Symtropy #7230: broader morphism hierarchy and quotient/simulation research.
- Symthaea #11171: production research-policy adapter.
