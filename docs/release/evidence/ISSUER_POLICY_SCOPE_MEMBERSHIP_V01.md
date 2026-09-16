# PHYS-EVID-03B1C — Deterministic locally-trusted-policy scope membership v0.1

Status: source-candidate theorem stacked on PHYS-EVID-03B1B.

Parent product candidate: corrected B1B branch `physics/issuer-policy-trust-bootstrap-v0.1`.
Design issue: #1152.

## Purpose

Separate deterministic issuer-policy membership evaluation from cryptographic evidence validity.

B1A defines exact canonical policy semantics. B1B establishes one explicit **local caller** trust selection over exact B1A bytes; it does not authenticate which operator/deployment principal made that selection. B1C consumes one such exact locally trusted snapshot and answers only whether one plain authorization query lies inside its scopes.

B1C does not prove that the query came from B0-valid physics evidence and does not upgrade B1B into deployment/operator-authorized policy. Deployments requiring authenticated bootstrap provenance must compose PHYS-EVID-03B1B2 (#1155).

## Core theorem

```text
LocallyTrustedIssuerAuthorizationPolicyV1
+ IssuerAuthorizationQueryV1          (plain caller-constructible data)
    -> IssuerPolicyScopeMatchV1
       | exact deterministic denial reason
```

The successful match token proves only exact scope membership under the borrowed locally trusted snapshot.

It is **not** `IssuerAuthorizedPersistentPhysicsEvidenceV1`.

## Query v1

```text
credential_id[32]
physical_authority_id: u128
claim_profile
session_profile
attestation_suite
crypto_policy_profile
```

All profile enums are the closed B1A v1 values. There is no wildcard field.

The query is intentionally plain/public. A later B1 composition must derive these fields internally from B0-verified canonical evidence and must not accept an independent caller-supplied B1C query as proof of issuer authorization.

## Deterministic evaluation order

The evaluator performs exactly:

1. binary-search the canonically sorted policy entries for exact `credential_id`;
2. absent -> `UnknownCredential`;
3. require status `Active`;
4. `Disabled` -> `CredentialDisabled`;
5. `RevokedCompromised` -> `CredentialRevokedCompromised`;
6. zero physical authority -> `ZeroPhysicalAuthorityId`;
7. require exact PhysicalAuthorityId membership;
8. require exact claim-profile membership;
9. require exact session-profile membership;
10. require exact attestation-suite membership;
11. require exact crypto-policy-profile membership;
12. only then construct `IssuerPolicyScopeMatchV1`.

The decision is independent of source map ordering, source file path, policy revision ordering, wall time, network arrival order, or credential lexical preference beyond exact canonical ID lookup.

## Successful match token

`IssuerPolicyScopeMatchV1<'a>` has private fields and retains:

- a borrow of the exact `LocallyTrustedIssuerAuthorizationPolicyV1` snapshot used for the decision;
- the exact copied query values.

It exposes read-only query/snapshot/revision accessors.

The token has no public constructor and no conversion to issuer-authorized evidence.

The borrowed snapshot remains B1B-local caller trust only unless a stronger B1B2 deployment-authorization theorem is composed.

## Exact snapshot identity

Exact canonical B1A bytes retained by B1B remain the policy-snapshot identity authority.

A future digest may index a decision, but digest equality does not replace exact canonical policy-byte equality.

`policy_revision` is administrative metadata only and is not trusted chronology or winner selection between snapshots.

## Error semantics

Closed v1 denial reasons:

```text
UnknownCredential
CredentialDisabled
CredentialRevokedCompromised
ZeroPhysicalAuthorityId
PhysicalAuthorityNotAllowed
ClaimProfileNotAllowed
SessionProfileNotAllowed
AttestationSuiteNotAllowed
CryptoPolicyProfileNotAllowed
```

Denial order is part of the deterministic evaluator contract.

## Layer placement

B1C lives in the top-level integration crate alongside B1B. It reuses:

- B1A closed enum/accessor semantics;
- B1B exact locally trusted snapshot wrapper.

It contains no signature verification, digest algorithm, Xenia key material, contradiction adjudication, replay store, ActionKey, or settlement logic.

## Required corpus

- exact active credential + all scopes -> match;
- either exact authority in a multi-authority scope may match;
- unknown credential -> `UnknownCredential`;
- disabled credential -> `CredentialDisabled`;
- compromised credential -> `CredentialRevokedCompromised`;
- zero authority -> `ZeroPhysicalAuthorityId`;
- wrong authority -> `PhysicalAuthorityNotAllowed`;
- wrong claim profile -> `ClaimProfileNotAllowed`;
- same exact query + same exact snapshot -> deterministic same query/snapshot result;
- revision-only snapshot change may alter snapshot bytes/metadata while leaving the same query semantically matchable;
- successful scope match is not called authenticated/issuer-authorized physics evidence;
- B1B local caller trust is not described as authenticated deployment/operator trust.

Singleton v1 session/suite/crypto-policy registries make “wrong but valid alternate enum value” impossible through the safe v1 enum API. The evaluator still performs exact membership checks so future closed enum expansion does not silently widen existing policy semantics.

## Relationship to final B1

Later local-profile composition must have the form:

```text
CryptographicallyValidEvidenceAttestationV1
    -> derive exact query fields internally

exact query fields
+ LocallyTrustedIssuerAuthorizationPolicyV1
    -> B1C match

same B0-valid attestation
+ same derived query
+ same B1C match
    -> IssuerAuthorizedPersistentPhysicsEvidenceV1
```

Never:

```text
caller-created query
+ B1C scope match
    -> issuer-authorized physics evidence
```

For deployments requiring caller/deployment-authenticated policy roots, final B1 must use the separate B1B2 `DeploymentAuthorizedIssuerPolicySnapshotV1` path rather than silently treating the local B1B type as stronger than it is.

## Qualification boundary

B1C is stacked over B1B/B1A source candidates. Its qualification lane may rerun predecessor source/corpora but must not retroactively claim their separate queued helpers passed.

## Non-claims

No B0 cryptographic validity, no proof that query fields came from canonical physics evidence, no authenticated bootstrap caller, no deployment authorization, no issuer-authorized evidence type, no trusted time, no historical key validity, no contradiction resolution, no replay-safe action execution, no settlement/custody/governance decision, and no consensus are established by PHYS-EVID-03B1C v0.1.
