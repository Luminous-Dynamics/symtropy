# PHYS-EVID-03B1A — Canonical Issuer Authorization Policy v0.1

Status: representation theorem only. Parent design: issue #1146 and PHYS-EVID-03B1/#1145.

## Theorem

This tranche defines one language-neutral canonical byte representation for issuer authorization policy snapshots. Successful parsing proves only that bytes satisfy the closed v1 grammar and semantic constraints. It does **not** authenticate the policy, establish a trust root, authorize a credential, or recreate any live physics-authority token.

## Canonical wire grammar

```text
ASCII "symtropy-physics-issuer-policy\0"
u32be(format_version = 1)
u64be(policy_revision)
u32be(entry_count)
entry * entry_count
```

Entries are strictly sorted by raw `credential_id[32]` bytes:

```text
credential_id[32]
u8 status
u32be(physical_authority_count)
u128be physical_authority_id * count
u32be(claim_profile_count)
u32be claim_profile_tag * count
u32be(session_profile_count)
u32be session_profile_tag * count
u32be(attestation_suite_count)
u32be attestation_suite_tag * count
u32be(crypto_policy_count)
u32be crypto_policy_tag * count
```

Every scope is non-empty and strictly increasing. Duplicates reject. The draft encoder may sort unordered source sets; the decoder never repairs non-canonical wire order.

## Closed v1 tags

```text
credential status
  1 Active
  2 Disabled
  3 RevokedCompromised

claim profile
  1 endpoint-sample-v1
  2 consecutive-presence-v1

session profile
  1 os-csprng-v1

attestation suite
  1 hybrid-ed25519-ml-dsa65-v1

Xenia crypto policy
  1 hybrid-pqc-auth-v1
```

There is no wildcard tag.

## Bounds

```text
entries                         <= 1024
physical authorities per entry <= 64
profile tags per scope          <= 8
```

Counts are checked before allocation. Unknown tags still reject even when count bounds pass.

## Physical-authority admissibility

`PhysicalAuthorityId = 0` rejects. The current qualified local authority allocator starts from the non-zero namespace, so canonical issuer policy does not admit a scope value outside the qualified producer image.

## Revision semantics

`policy_revision` is an administrative label only. It is not trusted time, freshness, chronology, or precedence. Changing only the revision changes canonical policy bytes but cannot change semantic identity of any physics claim.

## Equality

Exact canonical policy byte equality is the v1 policy-equality authority. A future digest may index snapshots, but digest equality does not replace exact bytes as the semantic equality rule.

## Trust boundary

The public parser returns only `DecodedIssuerAuthorizationPolicyV1`. There is no `TrustedIssuerAuthorizationPolicyV1` type or conversion in this tranche.

A later theorem may perform:

```text
canonical policy bytes
+ explicit trust bootstrap / policy-root verification
-> trusted issuer policy
```

but that authority does not come from this codec.

## Golden vector

The checked-in fixture freezes policy revision `7` with two credentials. Draft source order is deliberately non-canonical. The first canonical credential (`0x11` repeated 32 times) is Active and scopes physical authorities `0x10` and `0x20` plus both claim profiles. The second credential (`0x22` repeated 32 times) is Disabled and scopes authority `0x30` plus endpoint-sample-v1.

The exact canonical policy is **237 bytes**. Rust and the independent Python stdlib oracle must reproduce the same bytes.

## Required failure controls

Qualification must cover at minimum duplicate credential IDs, duplicate scope members, zero physical authority, unknown status/profile tags, empty scopes, unsorted wire entries/scopes, over-bound counts before allocation, truncation, and trailing bytes.

## Non-claims

No policy authenticity, policy trust root, B0 cryptographic validity, issuer authorization, trusted time, historical-key validity, contradiction resolution, replay-safe side effects, custody/delivery/settlement semantics, governance decision, or consensus is established here.
