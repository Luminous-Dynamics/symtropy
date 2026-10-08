# Durable Execution Transparency & Witness Boundary v0.1

Status: source hardening / durable lifecycle integration; not a production transparency-service integration.

## Purpose

The durable execution adapter composes five security planes:

- exact local journal integrity;
- external freshness authority;
- retained monotonic freshness state;
- authenticated lifecycle records; and
- an independent transparency log + witness quorum.

Every state-changing lifecycle transition admits only a **strictly newer** transparency checkpoint.

The boundary is deliberately dual-monotonic:

    durable journal transparency sequence
        +
    independently retained witness transparency sequence

A new transition must advance past both. Read-only recovery may re-verify the currently staged checkpoint without advancing retained witness state.

The intended role separation is:

    execution signer
        !=
    freshness authority
        !=
    transparency log authority
        !=
    transparency witnesses

Role separation is enforced cryptographically by rejecting public-key reuse across those trust roles.

## Checkpoint identity

TransparencyCheckpointUnsignedV1 commits:

- schema and VDS-bound algorithm;
- transparency log identity and epoch;
- transparency checkpoint sequence;
- exact journal namespace and deterministic seed;
- exact durable journal event count and head hash;
- concrete VDS tree size and SHA-256 root;
- exact witness-policy commitment; and
- exact predecessor checkpoint digest.

The log signs the canonical SHA-256 digest of those fields, including the concrete VDS tree head.

Witness signatures are detached from the checkpoint object. Each witness signs a domain-separated digest containing its own witness identity and the exact checkpoint digest.

An accepted checkpoint therefore binds:

    exact journal pre-state
    + exact transparency log identity
    + exact checkpoint lineage
    + exact witness policy
    + authenticated witness observations

The checkpoint is deliberately a statement about the **pre-transition** durable head. The lifecycle event that consumes it records that evidence and then advances the journal.

## Retained witness state

TransparencyWitnessSetV1 is deliberately not Clone, Serialize, or Deserialize.

Each retained witness records the minimum observed checkpoint state needed for replay and VDS continuity:

    checkpoint sequence
    checkpoint digest
    journal event count
    journal head hash
    VDS tree size
    VDS root hash

The retained witness set is also bound in-process to the concrete
TransparencyLogAuthorityV1 public-key commitment. Binding is idempotent for the
same authority root and rejects a different key under the same log identity/epoch.
Direct witness admission therefore cannot silently rotate the transparency-log
signing key while reusing the retained witness lineage.

Genesis is a deterministic semantic checkpoint derived from:

    log identity + log epoch + witness-policy commitment

For general evidence verification, a checkpoint may be idempotently re-verified against a witness's already-retained checkpoint.

For a **new durable transition**, two different monotonic boundaries are checked. The durable lifecycle journal requires the candidate checkpoint sequence to be exactly the next sequence after its persisted history, while retained witnesses require the candidate to be newer than every witness state used for admission. This prevents a previously consumed checkpoint from authorizing another state-changing lifecycle event while still allowing a witness to catch up after missed intermediate checkpoints when VDS continuity is proven.

The following fail closed:

    candidate sequence < retained sequence
        -> RollbackDetected

    candidate sequence == retained sequence
    + different checkpoint digest
        -> EquivocationDetected

    candidate sequence == retained sequence
    + same checkpoint digest
    + new durable transition
        -> ReplayDetected

    candidate sequence > retained sequence
    + VDS head is not a monotonic extension
        -> NonExtension / RollbackDetected

    candidate VDS tree grows from retained VDS head
    + consistency proof is absent or invalid
        -> VDS consistency failure

    candidate checkpoint sequence may exceed retained sequence by more than one
    + VDS continuity is proven
        -> witness catch-up is permitted

    durable lifecycle checkpoint sequence skips a number
        -> durable transparency lineage failure

## Quorum and independence

The witness policy is content-addressed and binds:

- policy identity;
- transparency log identity and epoch;
- witness identity;
- witness public key;
- independence domain;
- quorum threshold; and
- minimum independent-domain threshold.

Duplicate witness identities and duplicate public keys are rejected.

Every supplied witness signature is verified. A malformed or invalid extra witness signature is not silently discarded to salvage a convenient threshold.

The current v0.1 model supports quorum over configured independence domains, but it does **not** claim that different labels prove real-world organizational independence, geographic independence, or non-collusion.

## Durable lifecycle integration

The unified DurableExecutionSecurityContext now retains:

    sealed local exact-head witness
    external freshness authority
    retained freshness cursor
    current freshness attestation
    transparency log authority
    transparency witness policy
    retained transparency witness set
    current transparency checkpoint + witness signatures

The five lifecycle/recovery APIs use the same context.

For state-changing operations, the ordering is:

    verify local journal head
            ↓
    verify external freshness
            ↓
    verify retained freshness cursor
            ↓
    verify transparency checkpoint
            ↓
    verify witness quorum + continuity
            ↓
    construct lifecycle record containing consumed transparency evidence
            ↓
    durably append authenticated journal event
            ↓
    commit retained transparency witness state
            ↓
    advance local head witness + freshness cursor

The accepted transparency object is consumable: its witness predecessor state is captured during verification and the object is consumed only after the corresponding journal append succeeds.

The lifecycle event's execution-signature digest covers the complete persisted transparency evidence, so replacing that evidence changes the authenticated event payload.

The journal also retains a contiguous transparency history: sequence numbers advance one-by-one, each checkpoint names the immediately previous checkpoint digest, and the log/policy identity remains stable across the lifecycle journal. The first durable checkpoint must name the deterministic transparency genesis derived from log identity, log epoch, and witness-policy commitment. This gives the local journal a durable record of which external checkpoint sequences have already been consumed; it does not replace the independent witness memory.

## Persisted transparency evidence

Each lifecycle record persists:

    exact transparency checkpoint
    exact per-witness VDS consistency proof when tree growth requires one
    exact retained VDS frontier from which each witness continuity check was performed
    validated witness signatures
    canonical accepted witness identities
    canonical accepted independence domains

The older single shared-proof field remains only as compatibility metadata when all accepted witnesses used the same proof.

The persisted checkpoint is additionally tied to the lifecycle event's journal position:

    checkpoint.event_count == event.ordinal
    checkpoint.head_hash   == event.previous_hash

This means the evidence is explicitly for the exact pre-state of the event that consumes it.

The record is not treated as a substitute for retained witness state. The persisted event is an authenticated audit receipt; the independently retained witness state is still the anti-rollback/non-equivocation memory.

## Crash/restart boundary

The retained witness object remains intentionally non-serializable for ordinary in-process use.

A separate `TransparencyWitnessStateStore` boundary now defines restart-resistant storage semantics. Its backend must:

    durably retain the complete witness snapshot
    + provide linearizable load
    + provide atomic compare-and-swap over the whole snapshot
    + reject stale generation/snapshot writers
    + commit the replacement before reporting success

The generation counter is concurrency metadata, not a cryptographic freshness proof.

The lifecycle integration uses this store in the following order:

    verify candidate against retained witness state
        ↓
    durably append authenticated lifecycle event
        ↓
    atomically CAS the external witness snapshot
        ↓
    commit in-process witness state
        ↓
    advance local head/freshness witnesses

The two resources are intentionally not presented as one atomic transaction. If the journal append succeeds but external witness CAS fails, the security context becomes **fenced** and rejects subsequent state-changing operations rather than continuing with stale witness memory. This leaves the durable event intact while making reconciliation an explicit recovery operation.

External snapshots include the policy commitment, concrete log-authority commitment, canonical witness set, checkpoint lineage, journal head/count, and VDS frontier for every retained witness. A snapshot can therefore restore in-process witness state after restart, but the security claim still depends on the backend's actual durability and linearizable CAS semantics.

A backend that is merely a local file, eventually-consistent object store, or caller-controlled JSON document must not be described as an independent anti-rollback authority.

Therefore this tranche establishes:

    no silent in-process witness rollback
    no replay of an already-consumed transparency sequence
    durable authenticated record of which checkpoint was consumed
    an explicit restart-resistant witness-memory interface
    fail-closed fencing when the external witness authority cannot be advanced

It does **not** establish:

    that any particular backend is independently operated
    hardware rollback resistance
    cross-resource atomicity between journal and witness store
    automatic recovery after witness-store divergence

A runtime restart with only the execution journal must not reconstruct an external witness's memory. The journal now prevents replay of the last already-consumed transparency sequence, but it cannot establish that a fresh witness object has observed the intervening checkpoint history. The in-process witness can catch up across missed checkpoint sequences while it retains its prior VDS head; after restart, the next recovery layer still needs an independently retained witness/checkpoint store or equivalent external continuity authority.

This is analogous to the existing freshness cursor boundary: durable journal data can document what happened, but it must not become the sole authority for reconstructing an external anti-rollback memory.

## Failure taxonomy

| Observation | Meaning |
| --- | --- |
| Invalid local chain | local integrity failure |
| Older external sequence | freshness rollback |
| Higher sequence with wrong durable predecessor | durable transparency non-extension |
| Same transparency sequence with different digest | transparency equivocation evidence |
| Reuse of an already-consumed sequence for a new transition | transparency replay |
| No newer checkpoint observed | freshness/transparency unavailability or possible freeze |
| Quorum below threshold | insufficient corroboration |
| Quorum exists only inside one configured domain | insufficient independence |
| Restart without retained witness state | fail-closed recovery boundary |

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

This design is deliberately narrower than SCITT.

RFC 9943 requires a SCITT transparency VDS to support append-only history, non-equivocation, and replayability. The current Symtropy boundary supplies a durable execution checkpoint plus independently retained witness continuity, VDS consistency checking, and a service-neutral inclusion-evidence precursor. It still does not implement a SCITT receipt, C2SP wire encoding, or a public VDS. https://www.rfc-editor.org/info/rfc9943/

RFC 9162 defines Merkle consistency proofs that demonstrate that a newer tree contains the older tree as a prefix. The repository now has SHA-256 consistency and inclusion primitives, with tests spanning many tree shapes and corrupted proof/root/index/size inputs. The checkpoint carries the concrete VDS tree size/root; witness admission enforces VDS continuity against each retained witness and permits catch-up across missed checkpoint sequence numbers when a valid consistency proof connects the retained and candidate tree heads. Durable lifecycle admission separately enforces contiguous checkpoint sequence/predecessor lineage. Service-neutral inclusion evidence can prove an exact entry against the signed tree head. https://www.rfc-editor.org/rfc/rfc9162.html

The current C2SP Transparency Log Witness Protocol has the closest architectural shape: a witness retains its latest verified checkpoint, requires a consistency proof for a newer checkpoint, and requires continuity checking plus durable persistence to be atomic. The current Symtropy witness set mirrors the retained-state and CAS boundary, but is intentionally still service-neutral and in-process. It is therefore **not** C2SP wire compatible yet. https://c2sp.org/tlog-witness

The current C2SP transparency-log policy model also makes known logs, known witnesses, and a quorum rule explicit. That maps naturally onto the present policy commitment and quorum layer. The configured independence-domain rule remains an application qualification requirement rather than a proof of real-world independence. https://c2sp.org/tlog-policy

C2SP timestamped witness cosignatures are a separate wire-level statement: the timestamp is included in the signed cosignature structure and signed message. The present Symtropy witness signature intentionally does not contain a timestamp and therefore is not a C2SP timestamped cosignature. A future adapter must construct the canonical checkpoint note, include the timestamped cosignature input, and treat timestamp acceptance as an explicit policy decision rather than silently turning an unsigned local clock into trusted time. https://c2sp.org/tlog-cosignature

TUF remains useful for the threat taxonomy because rollback and indefinite-freeze attacks are separate classes. This tranche provides positive rollback/equivocation evidence but does not convert absence of fresh evidence into proof of malicious freeze. https://theupdateframework.io/docs/security/

Sigstore likewise treats an append-only transparency log and independent monitoring as complementary controls. The current Symtropy boundary is the local admission-side analogue and does not claim global monitoring coverage. https://docs.sigstore.dev/about/security/

## Claim ceiling

A successfully executed integration can support claims such as:

    exact journal pre-state bound to an authenticated transparency checkpoint
    consumed checkpoint evidence durably authenticated in the lifecycle record
    witness-visible VDS lineage is non-equivocating within retained witness state
    retained witness state is bound in-process to one concrete transparency-log authority key commitment
    witnesses can catch up after missed checkpoint sequence numbers when VDS continuity is proven
    accepted quorum satisfies the configured witness/domain policy
    already-consumed transparency sequences cannot authorize a new durable transition
    durable transparency sequence history is contiguous and predecessor-linked
    witness-state commit is CAS-protected after the corresponding durable append

It must not silently promote those into:

    global transparency-service availability
    globally observed non-equivocation
    honest witnesses
    proof of real-world independence
    absence of collusion
    restart-resistant witness persistence
    hardware rollback resistance
    trusted time
    physical execution success
    semantic correctness of the underlying industrial process

## Adversarial corpus

The minimum regression corpus for this layer should continue to cover:

- valid new checkpoint;
- exact-head mismatch;
- same-sequence conflicting checkpoint;
- lower sequence;
- skipped durable checkpoint sequence;
- higher sequence with wrong predecessor;
- same checkpoint replay for a new transition;
- quorum below threshold;
- quorum entirely within one domain;
- duplicate witness identity;
- duplicate witness signature;
- invalid extra witness signature;
- log/freshness key reuse;
- witness/freshness key reuse;
- log/witness/execution-key reuse;
- policy/log epoch mismatch;
- policy commitment drift;
- concurrent witness-state change before commit;
- durable append failure before witness commit;
- journal tampering after evidence persistence;
- restart with journal-only state and no retained witness state;
- freshness checkpoint and transparency checkpoint bound to different exact journal heads.
- Merkle consistency proof path corruption.
- advertised Merkle root corruption.
- invalid tree-size relationship.
- witness catch-up after missed checkpoint sequence numbers with a valid VDS consistency proof.
- witness catch-up without the required VDS consistency proof.
- durable lifecycle checkpoint sequence gap despite valid signatures.
- external witness store generation rollback.
- external witness store snapshot mismatch under a matching generation.
- stale accepted checkpoint against changed external witness state.
- successful journal append followed by failed external witness-state CAS.
- restart restoration from an externally retained witness snapshot.

## Next implementation frontier

The semantic composition and protocol-shaped RFC 9162-style SHA-256 consistency/inclusion primitives are now implemented. Consistency verification is part of witness admission; `TransparencyInclusionEvidenceV1` is the separate offline inclusion-proof boundary.

The checkpoint commits a concrete VDS tree-size/root, and witness admission invokes the consistency verifier against the retained VDS head. Inclusion evidence binds an exact entry's RFC 9162 leaf hash to that signed tree head. Both remain service-neutral: the implementation still does not claim C2SP wire compatibility or a SCITT COSE receipt.

    semantic checkpoint + concrete VDS tree head
        ↓
    per-witness VDS continuity / catch-up
        ↓
    durable lifecycle sequence / predecessor admission
        ↓
    optional inclusion/non-inclusion proofs
        ↓
    SCITT/C2SP-compatible receipt/transport adapter

The clean architectural rule remains:

    protocol transport and VDS mechanics
        sit underneath
    the durable execution admission policy

The semantic witness layer now represents consistency evidence **per witness** rather than requiring one proof to serve an entire quorum. Different witnesses may legitimately retain different VDS tree sizes, so each evidence tuple binds:

    witness identity
    + witness checkpoint signature
    + consistency proof from that witness's retained VDS head

Quorum and independence-domain admission operate over independently verified tuples without requiring all selected witnesses to share one VDS frontier. The legacy shared-proof API remains as a compatibility wrapper that expands one proof to each supplied witness.

Persisted lifecycle evidence retains the exact per-witness proof tuples, while the older shared-proof field is retained only as a derived compatibility representation when every witness uses the same proof.

This is a liveness/interoperability refinement, not a relaxation of durable checkpoint sequence or predecessor checks. It keeps execution authority, freshness authority, transparency authority, and witness policy independently attributable.

The next implementation frontier is concrete external-backend qualification and wire interoperability. A future C2SP/RFC 9942 adapter must map the semantic per-witness evidence into the protocol's base64/note or CBOR/COSE structures without silently dropping journal, policy, or VDS bindings.

The repository now also exposes protocol-shaped Merkle proof bindings:
    RFC 9942 consistency content
        = older tree size + newer tree size + consistency path
    RFC 9942 inclusion content
        = tree size + leaf index + inclusion path

Those bindings are intentionally semantic rather than wire encodings. Their verification methods consume the tree sizes carried by the proof object itself, reducing the chance that a future transport adapter pairs a valid path with the wrong tree-size context. The consistency/inclusion binding types also retain the exact root hashes used for verification, so a proof cannot be validated successfully against a different root merely because a caller supplied a different root argument. The eventual CBOR/COSE adapter must still encode hashes as bstr and must authenticate the newer/inclusion root with the signed/detached payload exactly as the RFC requires.

A backend adapter should be qualified independently against crash consistency, durability, concurrent writers, generation rollback, snapshot corruption, and recovery-after-failure behavior. The repository's in-memory backend is only an adversarial semantic model and intentionally does not survive process restart.

C2SP policy remains a separate interoperability concern. Its current policy language supports nested named witness groups with all/any/numeric thresholds; the present Symtropy policy uses a flat quorum plus an application-specific minimum-independent-domain rule. A future policy adapter must model those semantics explicitly rather than treating the two policies as equivalent.

Protocol details were cross-checked against the current C2SP development specifications and RFC 9942 on 2026-10-08.
