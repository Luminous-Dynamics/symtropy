# SYM-EVAL-CORE-001 — Blind-run evidence chain v1

## Status

Draft research infrastructure. No performance or qualification claim follows from this document alone.

## Purpose

Provide one deterministic integrity and ordering substrate for blinded Symtropy/Symthaea evaluation runs.

The core answers only questions such as:

- which exact bytes were committed;
- what semantic role those bytes had in the receipt;
- which schema/producer identity was bound to them;
- which event preceded which later event;
- which A/B output stages were required **before execution began**;
- whether each trial executed those stages in the precommitted order;
- whether those required A/B outputs were committed before reveal;
- whether a revealed binary assignment opens the earlier hidden commitment;
- whether one authoritative post-reveal oracle and one terminal score were committed.

It does **not** decide whether a visual belief, tracker output, sensor frame, or score is correct.

## Placement

V1 lives in `symtropy-game-state::eval_evidence` because that package already owns deterministic identifiers, causal events, SHA-256, and hash-chain machinery without Bevy runtime dependencies.

The existing gameplay `EventChain<T>` is not reinterpreted as evaluation evidence. It hashes serde JSON for save/replay use. CORE-001 uses an independent canonical binary encoding and separate domain tags.

## Digest algorithm

V1 uses SHA-256 through the already-locked `sha2 = 0.10` dependency.

Algorithm identity is explicit as `Sha256V1`; a future version may add BLAKE3 without changing v1 receipt meaning.

Artifact commitments use:

```text
SHA256(
    u32_be(domain_length)
    || "sym-eval.artifact.v1"
    || u64_be(payload_length)
    || exact_artifact_bytes
)
```

The exact byte length is retained beside the digest.

## Canonical event encoding

Event digests use domain `sym-eval.event.v1` over a length-prefixed binary representation with:

- fixed-width big-endian integers;
- explicit enum tags;
- length-prefixed bounded UTF-8 strings;
- explicit option tags;
- exact 32-byte digests;
- canonical sorted artifact references;
- canonical sorted key/value bindings;
- committed event schema ID/version;
- no Rust `Debug` representation;
- no platform-native integer layout;
- no JSON whitespace/key-order dependency;
- no hash-map iteration dependency.

## Artifact references and authoritative roles

`ArtifactRefV1` binds:

- semantic role;
- exact digest and byte length;
- optional schema ID/version;
- optional producer repository + exact commit + component.

The semantic role is inside the event commitment. Reusing identical bytes as `sensor` versus `producer` therefore changes the event digest.

V1 also rejects multiple artifacts with the same role inside one event. If a stage contains many files, commit a manifest/bundle artifact or give each artifact an unambiguous role instead of creating competing authoritative values under one name.

Contract roles are single-assignment across the pre-run contract phase. A later `camera` or `scenario-contract` event cannot silently replace an earlier contract with the same role.

The hidden assignment commitment must reference the digest committed under the exact role:

```text
scenario-contract
```

A byte-identical camera/model/other contract cannot be substituted merely because its digest happens to match the requested value.

## Event order

The chain begins with exactly one genesis event. Every later event commits to its exact sequence and predecessor digest.

V1 event kinds are:

1. `Genesis`
2. `ContractCommitted`
3. `BlindingCommitted { commitment, requirements }`
4. `BlindOutputCommitted { trial, stage }`
5. `Reveal`
6. `PostRevealOracle`
7. `ScoreCommitted`

A successful completed evaluation follows the high-level state machine:

```text
Genesis
  ↓
one or more uniquely-role-bound contracts
  ↓
one blinding commitment + frozen stage profile
  ↓
A/B blind-stage pipelines
  ↓
one reveal
  ↓
one post-reveal oracle
  ↓
one terminal score
```

`ScoreCommitted` is terminal. No later evidence event belongs to the same completed v1 run.

## Blind stage profile

Blind stages are:

- `SensorInput`
- `ProducerOutput`
- `AdapterOutput`

`RevealRequirementsV1` is not only a reveal checklist. It is the **precommitted legal pipeline** for both A and B.

Only stages present in that profile may be committed. Each trial must advance through the profile monotonically.

For example, under:

```text
SensorInput
ProducerOutput
AdapterOutput
```

this is invalid:

```text
A/AdapterOutput
A/SensorInput
A/ProducerOutput
```

and this is also invalid:

```text
A/SensorInput
A/AdapterOutput
A/ProducerOutput
```

The valid order is:

```text
A/SensorInput
A/ProducerOutput
A/AdapterOutput
```

with B independently obeying the same ordering. A/B events may interleave globally as long as each trial's own pipeline remains valid.

`AdapterOutput` cannot appear in a profile unless `ProducerOutput` is also present.

A given `(trial, stage)` is a single authoritative commitment slot. Duplicate commitments fail closed rather than creating competing candidate outputs.

## Reveal policy is precommitted

`RevealRequirementsV1` is committed **with the blinding commitment, before any blind output is accepted**.

This closes an important downgrade path. A run cannot begin intending to require:

```text
SensorInput + ProducerOutput + AdapterOutput
```

and later reveal under a weaker producer-only policy because one adapter output failed to materialize.

The reveal event does not supply or replace the requirements. It can only demonstrate that the already-committed pipeline was completed for A and B.

Examples:

### Raw tracker profile

```text
SensorInput
ProducerOutput
```

for both A and B.

### Full integrated belief profile

```text
SensorInput
ProducerOutput
AdapterOutput
```

for both A and B.

Distinct profiles should have explicit run bindings/profile identity at the higher evaluation layer.

## Blinding assignment commitment

The core treats branch meaning generically:

- `Variant0InA`
- `Variant0InB`

The evaluator layer defines what Variant0 means outside the integrity kernel.

The public pre-reveal commitment binds:

- experiment ID;
- the exact already-committed `scenario-contract` digest;
- hidden binary assignment;
- 32-byte private nonce.

The nonce type rejects obvious degenerate sentinel patterns. This is **not** an entropy proof. Production runners remain responsible for a CSPRNG and secret handling.

## Commit-before-reveal

Reveal is accepted only when:

- the chain contains the matching prior blinding commitment;
- the reveal recomputes that commitment exactly;
- every stage frozen in the pre-run profile has exactly one committed A output and one committed B output earlier in the same chain;
- each A/B pipeline was committed in the predeclared stage order.

This property comes from hash-chain ancestry, not wall-clock timestamps.

A blind output appended after reveal is rejected.

Oracle and score artifacts are rejected before reveal.

## Post-reveal authority slots

V1 permits exactly one authoritative `PostRevealOracle` event after reveal.

`ScoreCommitted` requires that oracle event first and may occur exactly once. The score is terminal for the v1 chain.

This prevents receipts such as:

```text
oracle-v1
score-favorable
oracle-v2
score-different
```

from leaving downstream consumers to decide which result was authoritative.

If multiple metric files are part of one planned result, bind them inside the single score event using distinct artifact roles or a committed score manifest.

## Canonicalization policy

Artifact and binding input vectors are semantically unordered and sorted before hashing.

- exact duplicate artifact refs are rejected;
- duplicate artifact roles within an event are rejected;
- duplicate contract roles across contract events are rejected;
- duplicate binding keys are rejected;
- event order is never sorted or normalized;
- changing event order changes or breaks the chain.

## Frozen artifact test vector

For exact artifact bytes `abc`, v1 domain-separated SHA-256 is:

```text
a832bad61f1c5acedb7b543269ae5274c9e1637c78d97230f02b92d21760d639
```

The source tests enforce this vector.

## Negative controls

The focused tests cover:

- one-byte artifact mutation changes digest;
- artifact role changes event commitment;
- unordered artifact/binding input canonicalizes deterministically;
- duplicate event artifact roles fail closed;
- duplicate pre-run contract roles fail closed;
- a hidden assignment commitment cannot point at a non-`scenario-contract` role;
- obvious degenerate nonces are rejected;
- wrong assignment cannot open the commitment;
- wrong nonce cannot open the commitment;
- `AdapterOutput` without `ProducerOutput` in the profile is rejected;
- stages outside the precommitted profile are rejected;
- per-trial blind stages cannot execute out of order;
- a missing required A/B output blocks reveal;
- duplicate `(trial, stage)` output is rejected;
- score before oracle is rejected;
- duplicate oracle is rejected;
- duplicate score is rejected;
- parent-digest tampering is rejected;
- sequence rollback is rejected;
- blind output after reveal is rejected;
- a complete three-stage A/B chain verifies through terminal scoring.

## Integration targets

### SYM-EVAL-000B

Commit the raw tracker stage profile before running A/B. Post-hoc target↔track oracle association and metrics occur only after reveal.

### SYM-EVAL-001B

Commit the camera/render contract, then A/B sensor artifacts as `SensorInput` before the producer stage.

### SYM-EVAL-001C

For full belief evaluation, precommit the three-stage profile and commit A/B Symthaea belief exports plus A/B adapter/transcript outputs before reveal.

### SYM-EVAL-001D

Each executed matrix cell/run has its own terminal chain head. Aggregate reports reference immutable run heads instead of copying unbound metrics.

## Receipt claim levels

Keep these distinct:

1. **Integrity** — bytes, roles, schemas, identities, contracts, ordered stage pipeline, reveal, oracle, and score ancestry verify.
2. **Translation** — an adapter mapped producer state without semantic violations.
3. **Performance** — the post-reveal scorer produced metrics for the exact committed run.

A valid integrity chain is not a benchmark PASS.

## V1 persistence boundary

The source tranche defines and validates the in-memory canonical commitment/state machine. It does not yet claim crash-safe append persistence, validated receipt-file decoding, or resumable chain recovery across process restarts.

Those belong to CORE-002 so persistence bugs cannot quietly change the v1 digest/state semantics.

## Security / nonclaims

V1 provides tamper-evident SHA-256 commitments and causal ordering inside the qualified receipt model.

It does not establish:

- signer/operator authenticity;
- CI-runner authenticity;
- sensor-device authenticity;
- absence of side channels;
- nonce entropy beyond obvious-pattern rejection;
- crash-safe persistence;
- semantic correctness of committed artifacts;
- statistical validity of an experiment plan;
- object permanence;
- causal reasoning;
- visual competence;
- physical safety.

CORE-002 adds durability/recovery, CORE-003 addresses capability isolation, and CORE-004 addresses scientific preregistration. A later signature/attestation layer may bind an operator, CI identity, or Xenia evidence signature to a v1 chain head without changing the underlying v1 digest meaning.
