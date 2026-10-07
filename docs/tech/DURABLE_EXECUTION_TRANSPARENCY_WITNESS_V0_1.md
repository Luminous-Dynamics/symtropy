# Durable Execution Transparency & Witness Boundary v0.1

Status: source hardening / semantic boundary; not a production transparency-service integration.

## Purpose

The durable execution adapter already separates:

- exact local journal integrity;
- external freshness authority;
- retained monotonic freshness state; and
- authenticated lifecycle records.

The next failure mode is external non-equivocation. A correctly signed external freshness checkpoint can still be shown inconsistently to different relying parties unless an independently retained party remembers which checkpoint history it previously accepted.

This document freezes the narrow boundary added by the symtropy_bootstrap_persistence::transparency module:

    execution signer
        !=
    freshness authority
        !=
    transparency log authority
        !=
    transparency witnesses

The implementation is deliberately service-neutral. It does not implement HTTP transport, a Merkle VDS, a public log, trusted time, or physical execution.

## Checkpoint identity

TransparencyCheckpointUnsignedV1 commits:

- schema and algorithm;
- transparency log identity and epoch;
- transparency checkpoint sequence;
- exact journal namespace and deterministic seed;
- exact durable journal event count and head hash;
- exact witness-policy commitment; and
- exact predecessor checkpoint digest.

The external log signs the canonical SHA-256 digest of those fields.

The witness signatures are detached from the checkpoint object. Each witness signs a domain-separated digest containing its own witness identity and the exact checkpoint digest.

An accepted checkpoint is therefore not merely a statement that the log says it is current. It is:

    exact journal head
    + exact transparency log identity
    + exact checkpoint lineage
    + exact witness policy
    + authenticated external observations

## Retained witness state

TransparencyWitnessSetV1 is deliberately not Clone, Serialize, or Deserialize.

Each retained witness records only the minimum state needed for continuity:

    checkpoint sequence
    checkpoint digest
    journal event count
    journal head hash

Genesis is a deterministic semantic checkpoint derived from:

    log identity + log epoch + witness-policy commitment

A later candidate is accepted only when the counted witness can establish one of:

1. Idempotent replay — the checkpoint is exactly the checkpoint already retained.
2. Forward extension — the candidate sequence is higher and names the exact retained checkpoint as its predecessor.

The following fail closed:

    candidate sequence < retained sequence
        -> RollbackDetected

    candidate sequence == retained sequence
    + different checkpoint digest
        -> EquivocationDetected

    candidate sequence > retained sequence
    + wrong predecessor
        -> NonExtension

This makes same-sequence conflict evidence materially different from ordinary stale absence.

## Quorum and independence

The witness policy is itself content-addressed and binds:

- policy identity;
- transparency log identity and epoch;
- witness identity;
- witness public key;
- independence domain;
- quorum threshold; and
- minimum independent-domain threshold.

Duplicate witness IDs and duplicate public keys are rejected.

Every supplied witness signature is verified. An invalid, malformed, or unknown extra signature is not silently ignored to salvage a threshold.

This prevents the verifier from selecting a convenient valid subset out of a contradictory evidence bundle.

The current v0.1 model supports a quorum over independent domains, but it does not claim real-world independence merely because two configured domains have different labels.

## Durable ordering

The semantic lifecycle is:

    verify local journal head
            ↓
    verify external freshness
            ↓
    verify freshness cursor
            ↓
    verify transparency checkpoint
            ↓
    verify witness quorum / continuity
            ↓
    durable journal append
            ↓
    commit accepted transparency state
            ↓
    advance execution/freshness state

The accepted transparency object is consumable: commit_after_durable_append takes ownership of it.

Before committing, the witness set performs a compare-and-set style recheck of every predecessor state that was used during verification. A concurrent witness-state change therefore rejects the commit rather than overwriting newer state.

This is deliberately analogous to the already-established rule that freshness state must not advance when the durable journal append fails.

## Crash/restart boundary

This semantic tranche does not yet persist witness state into a platform-resistant external store.

Therefore it establishes:

    no in-process silent witness rollback

but not:

    restart-resistant witness rollback protection

The next durable integration should record the accepted checkpoint identity in the authenticated lifecycle journal and reconstruct witness state from a separately trusted checkpoint store. The lifecycle journal should never treat a caller-supplied serialized witness cursor as authoritative merely because its bytes are intact.

## Failure taxonomy

The intended distinctions are:

| Observation | Meaning |
| --- | --- |
| Invalid local chain | local integrity failure |
| Older external sequence | freshness rollback |
| Higher sequence with wrong local predecessor | external non-extension |
| Same external sequence with different checkpoint digest | external equivocation |
| No newer evidence within freshness policy | freshness unavailable / possible freeze |
| Conflicting witness-visible roots | split-view / transparency equivocation |
| Quorum below threshold | insufficient corroboration |
| Quorum exists only inside one independence domain | insufficient independence |

In particular:

    no newer checkpoint observed
        !=
    equivocation

    equivocation
        !=
    compromise

    witness outage
        !=
    proof of malicious freeze

## Relationship to transparency standards

This design is intentionally narrower than SCITT.

RFC 9943 requires a transparency service's verifiable data structure to provide append-only history, non-equivocation, and replayability. It also defines receipts and leaves consistency/inclusion proof mechanisms to the underlying VDS. The current Symtropy boundary implements the external checkpoint + independent witness continuity part only; it does not yet provide a SCITT receipt or public VDS consistency proof.

RFC 9162 defines Merkle consistency proofs for establishing that a newer tree contains the older tree as a prefix.

The current C2SP Transparency Log Witness Protocol is especially relevant to the next integration step: a witness retains its latest verified checkpoint and requires a Merkle consistency proof before accepting a newer checkpoint; same-size conflicting roots are rejected, and the witness update must be atomic with respect to the continuity check.

C2SP's policy model also makes witness quorum and witness grouping explicit. This maps cleanly onto the present quorum + minimum independent domains model, while the real-world independence assumption remains a separate qualification obligation.

TUF remains a useful threat-model reference because rollback and indefinite-freeze attacks are distinct. This tranche addresses non-equivocation/rollback evidence; it does not turn a silent source into proof of malicious freeze.

Sigstore's security model likewise treats an append-only transparency log and independent monitoring as complementary protections: a log can provide durable evidence, but monitoring is important for detecting inconsistent views.

## Claim ceiling

A future successfully executed integration may support claims such as:

    exact journal head bound to an authenticated transparency checkpoint
    witness-visible checkpoint lineage is non-equivocating
    accepted quorum satisfies the configured independence-domain policy
    checkpoint admission is state-CAS protected before durable commit

It must not silently promote those into:

    global liveness
    honest witnesses
    absence of collusion
    hardware rollback resistance
    trusted time
    physical execution success
    semantic correctness of the underlying process

## Next implementation frontier

The next concrete integration should connect this semantic boundary to the existing DurableExecutionSecurityContext without replacing its freshness cursor.

The intended composition is:

    DurableExecutionSecurityContext
    ├── sealed local exact-head witness
    ├── external freshness authority
    ├── retained freshness cursor
    ├── current freshness attestation
    ├── transparency log authority
    ├── retained transparency witness set
    └── current accepted transparency checkpoint

The accepted transparency checkpoint identity should then be persisted inside the authenticated Pending/terminal lifecycle record so restart recovery can reconstruct exactly which external checkpoint was consumed for each durable transition.

Only after that composition exists should the repository consider a concrete network transparency adapter or SCITT-compatible receipt format.

## References

- RFC 9943 — An Architecture for Trustworthy and Transparent Digital Supply Chains: https://www.rfc-editor.org/info/rfc9943/
- RFC 9162 — Certificate Transparency Version 2.0: https://www.rfc-editor.org/rfc/rfc9162.html
- C2SP Transparency Log Witness Protocol: https://c2sp.org/tlog-witness
- C2SP Transparency Log Policy: https://c2sp.org/tlog-policy
- TUF Security: https://theupdateframework.io/docs/security/
- Sigstore Security Model: https://docs.sigstore.dev/about/security/