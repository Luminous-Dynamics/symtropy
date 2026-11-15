# Maritime qualification scope calculus v0.1

**Owner:** MARITIME-QUALIFICATION-SCOPE-001 #5219  
**Parent:** MARITIME-OBSERVABILITY-001 #4609 / PR #4611  
**Status:** executable synthetic contract; no physical qualification asserted  
**Version:** 0.1.0

## Why this is separate from dependency impact

The existing `qualification_impact` evaluator computes the transitive impact closure after an artifact changes. This evaluator asks whether one previous qualification supports one new request, for the exact asset, configuration, regime, time and dependency set. They compose but do not replace one another:

1. A change-set produces an impact plan.
2. This scope compiler evaluates whether a requested capability remains supported, is outside scope, is stale, is unknown/conflicted, or needs requalification.
3. Any `REQUALIFIED` result is backed by an exact model-retention receipt; it is not silent inheritance.
4. The owning engineering/authority system remains responsible for review and any subsequent record mutation.

## Dispositions

| Disposition | Meaning |
|---|---|
| `SUPPORTED` | Exact asset/configuration, regime, validity time and all required dependency digests match a qualified prior record |
| `REQUALIFIED` | A model changed, but an independently reviewed retention receipt proves the exact prior capability/regime set survived the frozen update suite |
| `REQUALIFICATION_REQUIRED` | A configuration or non-model prerequisite changed, a model changed without sufficient retention evidence, or a prerequisite was revoked |
| `OUT_OF_SCOPE` | Different asset or a requested operating regime outside the declared qualification envelope |
| `EXPIRED` | The qualification or a required time-bounded evidence item has expired |
| `NOT_EVALUATED` | Required evidence is absent, unknown, unverified or malformed |
| `CONFLICTED` | A required evidence source explicitly reports conflicting authoritative states |

These states are intentionally not collapsed into a confidence score. In particular, missing evidence cannot be interpreted as healthy, and an out-of-scope condition is not a classifier error.

## Scope calculus

The support predicate is conjunctive:

`SUPPORTED = A \land C \land R \land T \land D \land P`

where:

- (A): exact asset identity;
- (C): exact installed-configuration digest;
- (R): requested regime belongs to the qualified regime set;
- (T): evaluation time lies inside the qualification validity interval and all evidence validity windows;
- (D): every required dependency is present, qualified and digest-identical;
- (P): physical-claim evidence has the required measurement provenance and independence review.

A failed prerequisite does not get averaged away by other evidence. The evaluator uses explicit precedence so that asset/regime scope, expiry, conflict, unknown evidence and digest changes remain distinguishable.

### Capability-specific prerequisite sets

The v0.1 compiler now supports an optional `required_dependency_ids` array on each qualification. When present, it must be a non-empty, unique set drawn from the seven registered dependency classes, and it must exactly match the qualification's dependency records and the request's evidence map. An absent required item or an extra/unregistered item is not silently ignored.

When `required_dependency_ids` is omitted, the original v0.1 behavior remains unchanged: all seven dependency classes are mandatory. This preserves the existing frozen cases and callers.

The opt-in form allows separate capabilities to declare different prerequisite sets without inventing dummy dependencies. For example, a wind-monitoring capability may require sensor topology, calibration and timebase, while a VMG route capability may additionally require a source-bound polar and the applicable model/evaluator/validation profile. A capability-specific profile still composes this compiler; it does not create a second qualification engine.

This is a structural scope feature only. It does not prove that the chosen prerequisites are scientifically sufficient for a real vessel; that mapping remains the domain profile's reviewed engineering responsibility.

## Model update retention

A model change is the only dependency change that may yield `REQUALIFIED` in v0.1. Its receipt must bind the previous qualification, old/new model hashes, exact scope hash, frozen retention suite, validation-profile digest, independent-review status, exact capability and regime sets, unknown-fault rejection, calibration retention and a bounded worst-case regression.

The compiler requires set equality for both capabilities and regimes. It cannot certify scope expansion. A widened or incomplete receipt remains `REQUALIFICATION_REQUIRED`.

The receipt is metadata, not a cryptographic attestation. The validator does not prove the reviewer’s identity, signature, independence or authority. That must be verified by the provenance/authority layer before any production use.

## Evidence classes

Synthetic known-truth cases exercise evaluator semantics only. For a claim marked `PHYSICAL_ENGINEERING_CAPABILITY`, this contract requires measured/published-physical evidence for sensor topology, calibration, timebase and source payload; independently audited sensor-principle independence; a verified source payload; and an independently reviewed validation profile. Simulator-only or derived evidence cannot satisfy this physical gate.

## Frozen suite

Twenty cases exercise exact-scope reuse, asset/configuration changes, regime boundaries, missing/conflicted/expired prerequisites, model updates, retention-receipt scope expansion, excessive regression and simulated evidence presented as physical evidence.

Reproduce with:

```sh
cargo test --locked -p maritime-observability-evidence-validator --bin qualification_scope
cargo clippy --locked -p maritime-observability-evidence-validator --all-targets -- -D warnings
source_commit="$(git rev-parse HEAD)"
input="benchmarks/maritime-observability/qualification-scope-cases.v0.1.json"
digest="$(sha256sum "$input" | awk '{print $1}')"
cargo run --quiet --locked -p maritime-observability-evidence-validator --bin qualification_scope -- \
  "$input" --source-commit "$source_commit" --case-sha256 "$digest"
```

## Claim ceiling

A pass establishes deterministic software conformance to these synthetic scope rules. It does not authenticate the vessel asset register, configuration, sensor topology, calibration, clock certificate, data payload, model artifact, validation reviewer or physical engineering capability. No external qualification record is mutated; no maintenance, actuation, operating-mode or restriction authority is granted.
