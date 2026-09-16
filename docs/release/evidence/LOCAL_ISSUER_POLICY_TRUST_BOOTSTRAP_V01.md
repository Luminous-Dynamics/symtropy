# PHYS-EVID-03B1B — Local explicit issuer-policy trust bootstrap v0.1

Status: source-candidate theorem stacked on PHYS-EVID-03B1A.

Parent product candidate: `509b4ea5af6167218c6e0c4ecaafdaff520d0705` (#1147).
Design issue: #1149.
Stronger deployment/operator caller-authorization theorem: #1155 / PHYS-EVID-03B1B2.

## Purpose

Separate canonical policy representation from one explicit local caller trust decision.

B1A proves that bytes are one canonical issuer-authorization policy snapshot. B1B proves only that **some caller in the current process explicitly selected exactly one such snapshot as trusted for its local verifier context**.

B1B does **not** authenticate which human, operator, deployment principal, or module made that call. A reusable public library function cannot prove caller identity merely because it returns an opaque Rust type. That stronger provenance belongs to B1B2 (#1155).

The theorem is intentionally local. It does not claim remote consensus, global freshness, historical chronology, policy-root signatures, deployment authorization, or issuer authorization by itself.

## Authority split

```text
candidate bytes
    -> B1A canonical decode
    -> DecodedIssuerAuthorizationPolicyV1      (plain/untrusted)

explicit one-shot local caller action
+ exact candidate bytes
    -> B1A decode + exact round-trip equality
    -> LocallyTrustedIssuerAuthorizationPolicyV1
```

The `LocallyTrusted` prefix is part of the theorem boundary. It prevents downstream code from reading a B1B result as deployment/operator-authorized policy merely from the type name.

Forbidden shortcuts:

```text
parse bytes -> locally trusted policy
From<DecodedIssuerAuthorizationPolicyV1> -> locally trusted policy
TryFrom<Vec<u8>> -> locally trusted policy
serde deserialize -> locally trusted policy
file path / mtime / source label -> policy identity
```

Also forbidden is the stronger interpretation:

```text
public zero-argument bootstrap function
    -> proof of operator identity or deployment authorization
```

That implication is false in B1B v0.1.

## V1 bootstrap profile

Stable profile label:

```text
local-explicit-policy-bootstrap-v1
```

The integration layer exposes the explicit bootstrap call so the low-level `symtropy-physics` policy codec never promotes parsed bytes into locally trusted policy automatically.

The public call itself is intentionally only a local caller decision boundary. Deployments requiring authenticated operator/deployment provenance must compose B1B2 or a stronger policy-root theorem.

## Exact snapshot identity

`LocallyTrustedIssuerAuthorizationPolicyV1` retains:

- the exact canonical B1A bytes selected by the bootstrap action;
- the admitted decoded B1A policy semantics;
- the stable local bootstrap profile label.

Exact canonical byte equality remains the policy-snapshot identity theorem. No digest is required or accepted as a substitute.

The candidate bytes are decoded and then re-encoded before local trust promotion. The re-encoded bytes must be exactly equal to the supplied candidate bytes.

## One-shot local marker

`LocalIssuerPolicyTrustBootstrap` is deliberately:

- non-`Clone`;
- non-`Copy`;
- non-serializable;
- consumed by one trust-bootstrap attempt.

The public integration-boundary action is explicit:

```text
begin_local_issuer_policy_trust_bootstrap_v1()
    -> LocalIssuerPolicyTrustBootstrap
```

followed by:

```text
marker.trust_exact_policy_bytes(candidate)
    -> LocallyTrustedIssuerAuthorizationPolicyV1
```

This prevents accidental parse/deserialization paths from silently becoming local-trust paths. It does **not** make the marker an unforgeable operator capability: any code able to invoke this public integration API can begin a local bootstrap.

The fact that a caller invokes this action is the v1 local trust decision. It is not evidence that a particular human/operator authorized it or that another host, organization, or network agrees.

## Immutability / no silent reload

The locally trusted wrapper owns its exact canonical byte vector. It does not retain a path, file descriptor, watcher, mutable draft, or borrowed source buffer.

Changing a source file or caller buffer after bootstrap cannot mutate an existing locally trusted snapshot. Policy replacement requires a new explicit bootstrap action and produces a distinct wrapper.

`policy_revision` is retained through B1A semantics but remains an administrative label. A numerically larger revision does not prove later trusted time or precedence.

## Layer placement

B1B lives in the top-level application/integration crate rather than `symtropy-physics`.

Rationale: the low-level codec should establish canonical representation only; the integration layer is where an explicit local caller choice is represented.

This layer placement does not authenticate the caller. That is intentionally deferred to B1B2.

No new dependency is introduced. The integration crate already depends on `symtropy-physics`.

## Required adversarial corpus

- exact canonical B1A bytes + explicit local marker -> locally trusted wrapper;
- malformed/trailing B1A bytes -> reject before local trust promotion;
- caller source-buffer mutation after bootstrap -> existing trusted bytes unchanged;
- same exact canonical bytes bootstrapped twice -> same policy semantics/bytes;
- revision-only change -> different snapshot bytes without trusted-time claim;
- no public locally-trusted-policy constructor from decoded policy;
- no `From` / `TryFrom` / serde path to locally trusted wrapper;
- no mutable access to locally trusted policy or exact bytes;
- no path / filename / mtime / watcher API;
- public zero-argument marker mint is not described as proof of caller identity;
- no B0 signature verifier, issuer-scope authorization, contradiction resolution, or action execution in this module.

## Relationship to B1

A later local-profile PHYS-EVID-03B1 composition may accept:

```text
CryptographicallyValidEvidenceAttestationV1
+ LocallyTrustedIssuerAuthorizationPolicyV1
    -> IssuerAuthorizedPersistentPhysicsEvidenceV1
```

only for deployments whose trust policy explicitly treats B1B local caller selection as a sufficient root. Deployments requiring authenticated operator/deployment provenance should require a B1B2 `DeploymentAuthorizedIssuerPolicySnapshotV1` instead.

B1B does not perform membership/status checks itself.

## Qualification boundary

Until #1147/B1A itself has executable qualification evidence, this stacked B1B candidate must remain source-only. A B1B qualification lane may rerun B1A source/corpus beneath it, but that does not retroactively convert #1148 into a PASS.

## Non-claims

No caller identity, operator authorization, deployment authorization, policy-root authenticity, global policy freshness, trusted time, historical retired-key validity, B0 cryptographic validity, issuer authorization, contradiction resolution, replay-safe action execution, settlement/custody/governance decision, or consensus is established by PHYS-EVID-03B1B v0.1.
