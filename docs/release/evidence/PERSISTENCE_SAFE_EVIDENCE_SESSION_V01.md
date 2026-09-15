# PHYS-EVID-05 PERSISTENCE-SAFE EVIDENCE SESSION v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue: PHYS-EVID-05 / #1074.

Implementation profile: Symtropy root integration library stacked on the sealed live evidence authority from #1086.

## Purpose

Close the restart-aliasing gap between process-local live temporal identity and durable exported evidence identity without moving operating-system entropy or key management into `symtropy-physics`.

The v1 profile is:

```text
LocalEvidencePhysicsAuthorityWorld<D>
        ↓ consume + OS-CSPRNG bootstrap
PersistentEvidenceSession<D>
        owns
        ├─ QualifiedEvidenceSessionBinding
        │    ├─ EvidenceSessionId[32]
        │    ├─ PhysicalAuthorityId
        │    ├─ WorldGenerationId
        │    ├─ LocalTemporalIncarnationId
        │    └─ EvidenceSessionProfileV1
        └─ exact sealed live evidence facade
```

This binding supplies the durable session namespace that later canonical semantic evidence must carry or inseparably reference.

## Why this lives outside `symtropy-physics`

The physics crate proves live world/subject/temporal semantics. It should not become an OS entropy or key-management authority merely because exported evidence needs restart-safe names.

The root integration library already directly depends on:

```text
rand = 0.8
symtropy-physics
```

so v0.1 uses that existing boundary for the concrete bootstrap profile without changing Cargo dependencies or `Cargo.lock`.

This is an integration/security bootstrap theorem, not a new physics theorem.

## EvidenceSessionId

`EvidenceSessionId` is exactly 32 bytes.

The public safe API exposes read-only exact bytes but provides:

- no caller-selected constructor;
- no parser from UUID/text/hex;
- no `Default`;
- no Serde authority;
- no timestamp/PID/file metadata constructor.

Production minting occurs only while bootstrapping a session from an exact sealed live evidence facade.

V1 uses `rand::rngs::OsRng` and `try_fill_bytes`.

An all-zero 256-bit value is reserved as invalid. Bootstrap retries all-zero entropy up to a fixed internal limit and fails closed rather than accepting the sentinel.

The theorem treats 256-bit OS-CSPRNG collision probability as negligible for accidental persistent namespace reuse. It does not claim mathematical collision impossibility or cryptographic authentication.

## Binding fields

`QualifiedEvidenceSessionBinding` contains private fields:

```text
EvidenceSessionId
PhysicalAuthorityId
WorldGenerationId
LocalTemporalIncarnationId
EvidenceSessionProfileV1::OsCsprngBoundLiveIncarnation
```

The binding has no public detached constructor and is deliberately non-Clone/non-Serde.

The profile code is frozen as:

```text
OsCsprngBoundLiveIncarnation = 1
```

A later canonical codec must include this profile identity with the binding semantics.

## Ownership instead of detached association

V1 does not merely return a random session ID beside a still-free evidence facade.

`PersistentEvidenceSession::bootstrap(evidence)` **consumes** the exact `LocalEvidencePhysicsAuthorityWorld` and stores it inside the session object.

Therefore safe code cannot:

```text
bootstrap session S from live incarnation I
recover I while S remains valid
bind S to unrelated incarnation J
```

The live facade and its persistence binding remain one owned object for the session lifetime.

## Permitted live operations

The session exposes:

- read-only session/binding identity;
- immutable `evidence_authority()` projection for already-qualified observation capture;
- `last_qualified_step_stamp()`;
- authorized pure stepping;
- authorized callback stepping;
- consuming clean/quarantine exit to the namespace boundary.

It exposes no:

```text
&mut LocalEvidencePhysicsAuthorityWorld
into_evidence_authority() on a successful session
world_mut()
authority_world_mut()
DerefMut
session-id rebinding
```

The session ID and A/G/incarnation binding remain unchanged as qualified steps advance.

## Clean exit / restart boundary

Successful session exit consumes the entire session and delegates to #1086:

```text
PersistentEvidenceSession::into_namespace()
```

Clean state returns `LocalNamespacePhysicsAuthorityWorld`.

To establish another persistent session after that boundary, the caller must:

```text
namespace
→ fresh #1086 seal
→ fresh LocalTemporalIncarnationId
→ fresh PHYS-EVID-05 bootstrap
→ fresh EvidenceSessionId
```

Thus a session cannot be silently resumed under the same live incarnation after clean exit.

A process restart/re-adoption is likewise required by policy to create a new session unless a future authenticated restoration theorem explicitly establishes continuity.

## Tainted exit

If a qualified physics step was interrupted and #1086 is tainted, consuming the persistent session preserves the existing quarantine result:

```text
Err(LocalTaintedEvidenceAuthority)
```

A persistence session does not launder partial-step state back into a qualified namespace.

## Bootstrap failure

Entropy acquisition failure or repeated all-zero entropy returns `EvidenceSessionBootstrapFailure` containing the original exact sealed live facade.

No session binding is created on failure.

Returning the evidence facade on bootstrap failure is safe because no persistence session was established and no session ID was issued.

## Restart alias theorem

Persistent semantic identity must not rely on bare:

```text
A/G/live-incarnation/step
```

because live incarnation allocation is process-local.

The durable semantic namespace becomes at least:

```text
EvidenceSessionId
+ A/G
+ bound live temporal incarnation
+ theorem/profile-specific claim position
```

Two live executions that happen to repeat the same numeric A/G/incarnation/step values after restart remain distinct when their evidence-session bindings differ.

## Re-attestation law

The evidence session belongs to semantic claim identity; signer/key envelope identity does not.

A later Xenia re-attestation under a rotated authorized key may change authentication-envelope bytes without changing the underlying evidence-session/semantic claim identity.

PHYS-EVID-05 therefore contains no signer fingerprint or signature bytes in `EvidenceSessionId`.

## Authentication boundary

This theorem does **not** authenticate the session binding.

OS-CSPRNG bootstrap proves the concrete local minting profile and binding construction. PHYS-EVID-03 / #1065 must later authenticate:

```text
canonical session binding
+ canonical evidence claim
+ issuer/suite context
```

A random session ID by itself does not prove who produced it.

## Canonical encoding handoff

PHYS-EVID-02 / #1064 and PHYS-EVID-02A / #1101 should encode either:

1. the full qualified session-binding semantics; or
2. an inseparable canonical commitment/reference to those semantics.

The canonical payload must never treat `LocalTemporalIncarnationId` alone as a globally durable namespace.

## Required executable corpus

At minimum:

- bootstrap returns a nonzero 256-bit session ID;
- binding exactly matches the consumed live A/G/incarnation;
- profile identity equals v1 code 1;
- qualified stepping preserves the session ID and incarnation binding;
- clean exit/reseal preserves A/G but changes live incarnation and session ID;
- entropy failure returns the original live facade with unchanged A/G/incarnation;
- one all-zero entropy result is retried and never issued;
- repeated all-zero entropy fails closed without a binding;
- source audit proves no public `EvidenceSessionId` constructor/parser/default/Serde;
- source audit proves `QualifiedEvidenceSessionBinding` fields private and no detached constructor;
- source audit proves successful session exposes no `into_evidence_authority`, mutable inner evidence projection, or rebinding API;
- source audit proves public production bootstrap uses `OsRng` and not PID/time/mtime/process-local counters;
- #1086 sealed evidence regression remains green.

## Dependency status

The exact product is stacked on #1086 sealed qualified temporal evidence authority.

Therefore this product remains source-only until both:

```text
#1086 / #1087 predecessor qualification
this exact-head qualification
```

execute successfully.

No downstream persistent semantic codec may claim stronger authority merely because this source exists.

## Non-claims

No mathematical uniqueness guarantee, no signature/authentication, no issuer authorization, no key identity, no canonical semantic claim bytes by itself, no trusted timestamp, no restart continuity restoration, no world-generation global uniqueness, no complete structural validity, no consensus, no continuous occupancy, no dwell, no arrival, no custody, no delivery, and no settlement eligibility are established here.
