# Living World Information Policy Registry Content Identity V0

## Status

Normative authority contract for future Living World policy-registry implementations.

This document does **not** claim current Rust already satisfies the contract.

## Problem

A semantic registry key such as `(registry_id, version)` is useful for human-readable lineage and policy evolution, but it is not sufficient to identify the exact policy corpus.

Two independently constructed registries can reuse the same semantic key while containing different:

- process requirements;
- representation capabilities;
- closure-evidence records or revocation state;
- future spatiotemporal or error-bound semantics.

Inside one runtime instance, possession of a sealed registry object may be enough to prevent ordinary caller substitution. Across save/reload, caches, evidence capsules, prepared transactions, replay, multiplayer, or federation, however, the exact registry contents must be identifiable.

## Core invariant

**Canonical policy authority binds both semantic registry lineage/version and exact registry content identity. Same semantic key with different content is a different authority state and fails closed.**

## Registry authority stamp

A canonical registry should eventually expose a complete authority stamp conceptually containing:

- semantic registry key/version;
- canonical manifest/encoding version;
- exact content fingerprint;
- optional higher-layer predecessor/succession provenance.

Prepared realization, collapse, promotion, process-activation, and fidelity-selection plans bind this authority stamp rather than a numeric registry version alone.

## Canonical manifest

The exact content fingerprint must be derived from a deterministic canonical manifest, not insertion order or process memory layout.

The manifest includes at least:

1. manifest/schema version;
2. semantic registry key/version;
3. every registered process key and semantic version;
4. every process minimum authority requirement;
5. every ordered information requirement and evidence-acceptance policy;
6. every representation key/schema version;
7. every representation authority level;
8. every ordered capability/evidence claim;
9. every registered closure-evidence record and status;
10. any future spatiotemporal, closure-error, transition, or selection-policy metadata that changes process legality.

Ordered maps/sets may define canonical record order. Bootstrap insertion order must not alter the manifest.

## Fingerprint semantics

The fingerprint algorithm/encoding is versioned and deterministic.

Forbidden identities include:

- Rust `DefaultHasher`;
- pointer/object identity;
- nondeterministic map iteration;
- unspecified JSON serialization order;
- caller-supplied opaque tokens with no binding to actual registry contents.

`lifesim-core` need not become a PKI subsystem. It may instead define canonical manifest bytes/records and let a higher authority layer apply an approved fixed digest algorithm, provided the resulting fingerprint is unambiguously bound to the exact manifest.

## Equality and succession

Two registry instances represent the same exact policy authority only when their complete authority stamps match.

Therefore:

- same semantic key + same manifest -> same fingerprint;
- same semantic key + any policy/evidence difference -> different fingerprint;
- higher numeric version does not automatically authorize migration;
- equal numeric version does not imply equal contents;
- explicit migration/succession is separate from content equality.

## Prepared-plan rule

A future-bearing prepared plan records the registry authority stamp used to resolve process and representation profiles.

If the selected registry content changes before COMMIT, the plan is stale even when the semantic registry key/version remains numerically identical.

No partial mutation occurs.

## Save/reload rule

Persistent canonical state that depends on policy evaluation records enough registry authority identity to prove one of:

1. exact same policy corpus resumed;
2. an explicit versioned migration/requalification occurred before canonical execution resumed.

Reload may not silently reinterpret old process/fidelity decisions under a different registry corpus that reused the same semantic key.

## Cache and evidence rule

Any cache whose result depends on registry policy includes the content fingerprint in its key.

Living World Observatory evidence records the exact registry authority stamp so a future reviewer can determine precisely which process/capability/evidence corpus authorized the run.

## Composite authority interaction

A `CompositeCapabilityContext` binds the registry authority stamp under which its component sources were resolved.

Sources resolved under different incompatible policy corpora cannot be mixed merely because their individual representation keys look equal.

## Promotion and fidelity interaction

Promotion reachability, canonical fidelity selection, and atomic process activation all bind the exact registry authority stamp used to make the decision.

Changing the registry corpus requires re-evaluation of:

- enabled process requirements;
- representation/context sufficiency;
- spatiotemporal validity;
- transition reachability;
- closure acceptance;
- deterministic semantic selection.

## Required qualification fixtures

1. same registrations in different insertion order yield identical canonical manifest and fingerprint;
2. changing one process requirement changes the fingerprint;
3. changing one representation capability changes the fingerprint;
4. revoking one closure evidence lineage changes the fingerprint;
5. same semantic key with different contents fails exact-authority comparison;
6. a plan prepared under fingerprint A cannot commit under fingerprint B;
7. save/reload with the exact same registry yields the same authority stamp;
8. a changed policy corpus requires explicit migration/re-evaluation;
9. caches keyed under fingerprint A are not reused under B;
10. renderer/camera/hardware state cannot alter registry content identity.

## Non-goals

This contract does not define:

- who is authorized to publish registry policy;
- PKI or distributed trust;
- remote policy distribution;
- consensus over registry updates.

It defines exact local content identity and replay provenance only.

## Summary theorem

**A registry version says which policy lineage we intend to use. A content fingerprint proves which exact policy corpus we actually used. Canonical ecology binds both.**
