# Executable manufacturing assurance-case graph (v0.1)

This dependency-free Rust reference evaluates a typed assurance-case graph for a declared assessment scope and UTC time. It handles claim support, evidence tier, assumptions, defeaters, counter-evidence, validity intervals, scope and explicit intervention authority.

It is a strict *manifest checker*, not a theorem prover, evidence authenticator, safety certifier or autonomous controller.

## Build and run

```sh
rustc --edition=2021 -D warnings --test tools/manufacturing-assurance/assurance_case.rs -o /tmp/mfg-assurance-tests
/tmp/mfg-assurance-tests
rustc --edition=2021 -D warnings tools/manufacturing-assurance/assurance_case.rs -o /tmp/mfg-assurance
/tmp/mfg-assurance tools/manufacturing-assurance/fixtures/valid-assurance-case.tsv
```

Or run the full suite:

```sh
bash tools/manufacturing-assurance/run-tests.sh
```

## Record types

The format is UTF-8, pipe-delimited and intentionally simple for audit. A literal pipe is not allowed inside a field.

- `CASE|id|scope|as_of_utc|minimum_tier|top_claim`
- `CLAIM|id|scope|valid_from|valid_until|minimum_tier|ALL_OR_ANY|kind|action_key_or_dash|statement`
- `EVIDENCE|id|scope|valid_from|valid_until|tier|status|sha256|statement`
- `ASSUMPTION|id|scope|valid_from|valid_until|ACCEPTED_OPEN_REJECTED|statement`
- `DEFEATER|id|scope|valid_from|valid_until|statement`
- `AUTHORITY|id|scope|valid_from|valid_until|GRANTED_REVOKED_SUSPENDED|principal|action_scope`
- `EDGE|source_id|target_id|relation`

The assessment time and all validity intervals use canonical UTC `YYYY-MM-DDTHH:MM:SSZ`. Intervals are half-open: `[valid_from, valid_until)`.

Evidence tiers are T0 synthetic, T1 analytical/independent reference, T2 public measured data, T3 independent laboratory, and T4 industrial pilot. These are project evidence tiers, not an ISO or regulatory grading scheme.

## Graph and evaluation semantics

Relationships are directed from source to target:

- `SUPPORTS`: a subclaim or evidence supports a claim.
- `COUNTERS`: evidence counters a claim.
- `ASSUMES`: an explicit assumption conditions a claim.
- `DEFEATS`: a defeater challenges a claim.
- `DISCHARGES`: evidence is offered to discharge a defeater.
- `AUTHORIZES`: a named authority grant permits a specific intervention claim.

Each claim chooses a support strategy:
- `ALL`: every directly linked supporting claim/evidence must pass.
- `ANY`: at least one directly linked support path must pass.

A claim is:
- **QUALIFIED** when its declared support paths pass and no unresolved assumption or defeater remains.
- **CONDITIONAL** when support passes but at least one accepted assumption remains explicit.
- **BLOCKED** when evidence is missing, stale, unverified, under-tier, scope-incompatible, or when an assumption/defeater/authority requirement remains unresolved.
- **DEFEATED** when current verified counter-evidence or a rejected assumption defeats it.

A verified discharge is not silently inferred: an active defeater requires a current, verified evidence item at or above the claim's minimum tier. Intervention-authorization claims additionally require a current GRANTED authority node whose action scope matches exactly (or uses the explicitly broad `*` wildcard). The tool only evaluates declared authority; it does not grant authority.

## Scope policy

- GLOBAL evidence/claims/authority may support a narrower local claim.
- Local artifacts may not support a GLOBAL claim.
- Local evidence may only support a claim with the same scope.
- Scope compatibility is checked on each edge, not guessed from file names or labels.

## Evidence integrity boundary

The SHA-256 field is checked for 64 lowercase hexadecimal characters only. The checker does **not** read or hash referenced artifact bytes, validate signatures, authenticate the submitter, prove calibration traceability, or assess semantic relevance. An evidence status of VERIFIED is a declared admission state from the manifest. A production implementation must bind it to an independently authenticated evidence receipt and immutable artifact bytes.

The report prints statements and evidence digests so reviewers can inspect the argument, but text is still an assertion—not proof.

## Frozen fixture and tests

`fixtures/valid-assurance-case.tsv` includes:
- a composition-validity top claim;
- a supporting physics-validity subclaim;
- an intervention-authorization subclaim;
- verified T0 evidence;
- an accepted explicit assumption (so the top claim is CONDITIONAL);
- a named defeater that is discharged by verified current evidence;
- a time- and scope-bounded authority grant for `adjust_feed_rate`.

The tests mutate the case to create stale evidence, scope leakage, unresolved defeaters, counter-evidence, open/rejected assumptions, missing/revoked/wrong-action authority, insufficient tier, support cycles, duplicate IDs/edges, invalid digest/time, and non-verified evidence. Deterministic replay is also tested.

## Standards relationship

The node/edge concepts are informed by:
- ISO/IEC/IEEE 15026-2:2022 assurance-case structure and terminology;
- OMG SACM 2.2 for claims, argumentation, evidence references, assumptions/contexts, counter-evidence and defeated assertions;
- GSN and Claims-Arguments-Evidence (CAE) notations.

This reference is not a conformant implementation of SACM, GSN, CAE or ISO/IEC/IEEE 15026-2. No conformance claim is made. Its strict evaluation policy is an additional project-specific profile.

## Claim ceiling

This version checks structural completeness and declared policy consistency only. It does not prove that a claim is true, that the evidence is sufficient in a domain-expert sense, that an intervention is safe, or that a model is valid. Human/independent review and external measurements remain necessary.
