# AERO Observer-Coverage Qualification and Independent Replay Benchmark v0.1

**Tracker:** follow-on research to AERO temporal assurance #7455 / PR #7456; integrates with BASALT observer-coverage ownership (#9239)  
**Status:** research contract and synthetic benchmark proposal; not an observer implementation or physical qualification  
**Claim ceiling:** scoped synthetic coverage semantics and benchmark design  
**Owner boundary:** BASALT owns generic observer qualification; Mycelix/AeroCommons owns source-native receipts, checkpoints and artifact identity; Symtropy AERO consumes those qualified records and applies engineering-specific replay semantics  
**Oracle status:** `NOT_INDEPENDENTLY_REVIEWED`

## 1. Research objective

AERO v0.1 separates evidence-checkpoint integrity from observer coverage. That distinction is necessary but incomplete. A checkpoint can bind an evidence view, and a coverage receipt can state that an observer was qualified, without the replay contract fully specifying **what the observer was obligated to see, how that obligation was measured, and how omitted events are detected**.

This proposal turns observer completeness into a falsifiable, scoped qualification claim.

The central question is not:

> Is there a digest, receipt, or field saying coverage is complete?

It is:

> For this exact subject, acquisition interval, source/channel universe, observer configuration, evidence checkpoint and trust snapshot, what independent evidence supports the claim that every required observation opportunity was captured or that every gap is disclosed?

The governing principle is:

**Absence may be inferred only inside an explicitly closed, independently qualified observation boundary. Outside that boundary, absence is unknown.**

A coverage result is never global. `QUALIFIED_COMPLETE` means complete only for the declared scope, time interval, source roster, channel set, observer profile, event classes and checkpoint generation. It does not establish that the roster itself is complete unless the boundary that defines the roster is independently justified.

This work must compose BASALT's generic observer-coverage mechanism. It must not add a second observer, evidence ledger, event store, checkpoint service, provenance graph, PKI or qualification lifecycle to Symtropy.

## 2. The first paradigm shift: from a timestamp watermark to a vector evidence cut

A single receiver timestamp is not a sufficiently precise evidence coordinate for a distributed engineering system.

Independent instruments, calibration authorities, manufacturing execution systems, NDE workstations, environmental sensors and trust services may have:
- distinct clocks and clock uncertainty;
- independent event sequences and partitions;
- delayed or reordered transport;
- different replication and retention behaviour;
- causal dependencies that are not captured by wall-clock ordering.

The scalar (T_e) remains a useful conceptual cutoff, but its implementation should be a **checkpointed evidence-view object**, not an assumption that one timestamp totally orders all source histories.

Proposed research coordinate:

```text
ReplayCoordinate = {
  subject: {
    acquisition_interval: [earliest, latest],
    subject_identity_digest,
    configuration_digest
  },
  evidence: {
    checkpoint_id,
    checkpoint_root_digest,
    scope_manifest_digest,
    observer_profile_digest,
    coverage_receipt_digest,
    source_watermarks: [
      { source_id, partition_id, stream_epoch,
        last_contiguous_sequence, receiver_inclusion_offset }
    ],
    causal_cut_digest
  },
  trust: {
    trust_snapshot_id,
    trust_snapshot_digest,
    trust_policy_id,
    trust_policy_digest,
    trust_knowledge_cutoff
  },
  verifier: {
    profile_id,
    schema_digest,
    verifier_build_digest
  }
}
```

The source-watermark vector is an exact description of the receiver's admitted view. Each watermark is scoped to a stable source identity, partition and stream epoch. It must not be reduced to a maximum timestamp across sources.

A stream reset creates a new epoch; sequence numbers from different epochs must not be silently compared. A missing predecessor, unexplained sequence gap, fork, offset regression or unverified source-namespace alias invalidates the relevant completeness claim.

Where events depend on other streams, the checkpoint must also preserve causal dependencies (for example, predecessor commitments or referenced source offsets). A wall-clock sort is not a substitute for a causally consistent cut. Concurrent events may be incomparable; the replay must preserve that fact rather than inventing a total order.

This is a proposed extension to the evidence coordinate, not a claim that vector clocks, stream offsets or causal consistency are new inventions.

## 3. The second paradigm shift: completeness is a proof obligation, not a percentage

A coverage percentage can hide a critical omission. A measurement system can report 99.9% capture while missing the one instrument or channel that could carry a safety-significant revocation or measurement event.

Do not collapse the following into one scalar:

| Dimension | Required evidence |
|---|---|
| Boundary closure | Authority and versioned manifest defining the sources, partitions, channels, event classes and exclusions in scope |
| Capture opportunity | Expected source cadence, sequence/offset range, heartbeat and acquisition interval |
| Observer identity | Exact observer implementation, configuration, deployment and qualification profile |
| Continuity | Source-native sequence/predecessor evidence, stream epochs, gap/fork checks and receiver inclusion offsets |
| Loss accounting | Drop counters, overflow, backpressure, buffering and retention state |
| Independent reconciliation | Comparison with source-native counts/checkpoints or an independent audit/control path |
| Checkpoint binding | Digest and watermark vector for the exact committed evidence view |
| Time qualification | Source clock identity, synchronization state, precision and uncertainty |
| Challenge qualification | Controlled fault injection demonstrating that omissions, resets, forks and bypasses are detected |
| Exclusions | Explicit list and authority for every excluded source, event class, time range or channel |

The completeness contract has two separate claims:

1. **Boundary completeness:** the declared source/channel universe is a defensible description of everything that could materially influence this decision.
2. **Capture completeness:** the qualified observer captured or explicitly accounted for every required observation opportunity within that boundary.

A perfect capture of an incomplete source roster is not complete evidence. A complete source roster with a dropped final event is not complete evidence.

An average, confidence score or signed `complete=true` field cannot override a missing critical source, gap or failed reconciliation.

### 3.1 Closed-world and open-world semantics

The default is open-world.

A closed boundary requires:
- an identified owner of the source universe;
- a versioned source/channel manifest;
- a reasoned mapping from the engineering decision to required sources and event classes;
- explicit exclusions with authority, scope and effective interval;
- evidence that the manifest itself was current for the replayed configuration and interval;
- independently reviewable change capture when sources, firmware, routing, partitions or observer configuration change.

An unknown source, unregistered channel or material scope change must not be silently ignored. It makes the relevant boundary unresolved or incomplete.

A coverage certificate can support a negative claim only within its closed scope. It cannot prove the non-existence of an event outside the declared source universe.

## 4. Coverage receipt contract

The following is a proposed adapter-level contract. BASALT remains the owner of the generic qualification receipt; this specification defines the AERO inputs, semantics and challenge expectations.

### 4.1 Scope manifest

The manifest binds:
- product/article and configuration identity;
- acquisition interval and required time range;
- source identity and namespace binding;
- source family, partition, channel and event classes;
- expected cadence or source-native sequence/offset semantics;
- criticality and non-compensatory coverage requirements;
- observer and capture path identity;
- explicit exclusions and their authority;
- manifest version and canonical digest.

The manifest must not be generated solely from the set of sources that happened to appear in the captured data. That would make a missing source invisible by construction.

### 4.2 Per-source coverage record

For every required source/partition, retain:
- stable source identity and namespace binding;
- stream epoch and predecessor/checkpoint reference;
- expected and observed sequence/offset range;
- last contiguous sequence and first unresolved gap;
- receiver inclusion offset and receipt references;
- expected and observed heartbeat intervals;
- acquisition and source-clock uncertainty;
- drop, overflow, retry, quarantine and buffer-loss counters;
- independent source count/checkpoint where available;
- retention/replication status for the referenced bytes;
- source and observer configuration generations.

Unknown or unavailable fields remain unknown; they are not zero-filled.

### 4.3 Observer qualification receipt

The BASALT-owned receipt should bind:
- exact observer profile and build/configuration digests;
- deployment identity and observation interval;
- scope-manifest digest;
- per-source coverage vector or digest of the complete vector;
- evidence checkpoint root and watermark vector;
- independent qualification reference and test profile;
- challenge-suite version and result;
- known exclusions, limitations and unresolved channels;
- receipt issuer, authority scope and signature/receipt evidence.

The receipt must be verifiably bound to the same checkpoint, scope manifest, observer epoch and interval as the replay. A receipt for a different checkpoint or stale observer configuration cannot be reused.

A signed receipt establishes attribution and integrity of the assertion. It does not, by itself, establish that the assertion is true. The observer profile must have independent qualification evidence.

### 4.4 Dispositions

| Coverage state | Meaning | Replay consequence |
|---|---|---|
| `QUALIFIED_COMPLETE_FOR_SCOPE` | Closed scope; qualified observer; reconciled capture; no unresolved required gaps | Absence inference may proceed only within the exact scope |
| `INCOMPLETE` | Known missing source, channel, event range, interval or capture opportunity | `ABSTAIN` |
| `UNVERIFIED` | Scope, observer, receipt, reconciliation or qualification cannot be established | `TRUST_UNRESOLVED` |
| `INVALID` | Digest, binding, epoch, checkpoint or receipt integrity fails | `REJECT` |
| `OUT_OF_SCOPE` | Required evidence falls outside the declared source, time, configuration or event-class boundary | `ABSTAIN` or `TRUST_UNRESOLVED`; never infer absence |

`QUALIFIED_COMPLETE_FOR_SCOPE` is a prerequisite for a scoped negative inference. It is not a positive calibration, NDE, material or engineering qualification. The AERO evaluator must retain that distinction.

## 5. Frozen synthetic challenge corpus

The accompanying corpus contains positive, negative, incomplete and unresolved controls. It is deliberately synthetic and not yet independently reviewed.

Required adversarial families:

1. Missing required source despite a valid evidence checkpoint.
2. Missing sequence in the middle of an otherwise contiguous stream.
3. Missing final event after the last observed sequence.
4. Nonzero drop/overflow counter hidden by a healthy heartbeat.
5. Observer heartbeat outage overlapping the acquisition interval.
6. Source-native event count disagreeing with receiver count.
7. Forked stream with two successors for one predecessor.
8. Stream epoch reset without an authenticated epoch transition.
9. Receiver watermark regression or stale replica replay.
10. Unregistered source or channel discovered during independent reconciliation.
11. Open-world source roster presented as closed.
12. Critical channel excluded to improve aggregate coverage.
13. Partial time-range coverage represented as full-interval coverage.
14. Coverage receipt bound to a different checkpoint or scope-manifest digest.
15. Observer receipt signed but observer profile not independently qualified.
16. Coverage claim based solely on an observer's self-reported `complete` flag.
17. Namespace alias that moves an event outside the expected source roster.
18. Duplicate/correlated observer paths counted as independent capture channels.
19. Missing retention bytes after a nominally complete receipt.
20. A correct checkpoint whose contents omit an event because the capture path failed.

Positive controls are equally important:
- contiguous, independently reconciled source history;
- repeated delivery of identical event bytes, deduplicated without losing receipt references;
- concurrent independent sources with no causal dependency, represented without inventing a total order;
- explicit, authorized exclusion outside the decision's declared scope.

No evaluator should pass by rejecting every case. No synthetic result establishes real observer qualification.

## 6. Independent oracle and anti-gaming protocol

The expected dispositions must not be authored solely by the implementer of the evaluator being tested.

Before a confirmatory benchmark:
1. Freeze and hash the input corpus, source manifests, fault schedule and expected outputs.
2. Have a separate reviewer author or independently derive the expected dispositions from the normative contract.
3. Keep the expected-output file inaccessible to the implementation team during the confirmatory run where practical.
4. Use a second implementation or a manually derived finite-state oracle for boundary cases.
5. Mutate the evaluator and observer adapter deliberately: suppress one source, hide a sequence gap, falsify a drop counter, replay a stale checkpoint, substitute a namespace and alter an observer epoch.
6. Demonstrate that each mutation is detected by at least one independent check.
7. Publish all failures, abstentions, excluded cases and oracle disagreements.

A test that compares an implementation with fixtures authored from that same implementation is a consistency test, not independent ground-truth validation.

For a physical pilot, the oracle must include source-owner and laboratory evidence, independent acquisition/capture instrumentation, preregistered fault injection, and a reviewer independent of the developer of the observing system.

## 7. Benchmark metrics and proposed gates

### 7.1 Non-compensatory safety metrics

| Metric | Definition | Synthetic gate |
|---|---|---|
| False completeness acceptance | Incomplete/unverified/invalid challenge accepted as complete | Zero |
| Critical-source omission detection | Required critical source omitted but complete is reported | Zero misses |
| Gap/fork detection | Injected sequence gap or fork not surfaced | Zero misses |
| Scope-closure enforcement | Open/unverified scope treated as closed | Zero |
| Cross-checkpoint binding | Stale/mismatched receipt reused as current | Zero |
| Negative-claim containment | Absence inferred outside qualified scope | Zero |
| Raw evidence preservation | Capture/reconstruction mutates source bytes | Zero |
| Abstention correctness | Unknown/out-of-scope state silently promoted | Zero |

These are finite synthetic challenge gates, not statistical guarantees about every possible observer failure.

### 7.2 Diagnostic and operational metrics

Report per source, partition, event class and criticality:
- required, observed, reconciled and unresolved capture obligations;
- known omission recall under controlled fault injection;
- false alarm rate for gaps/forks;
- observer qualification coverage by fault family;
- source roster change-detection delay;
- time to detect sequence gaps, observer outage and retention failure;
- receipt latency and checkpoint interval;
- bytes retained per observation and per qualified source-hour;
- observer CPU, memory, bandwidth, storage and power;
- incremental and full replay cost;
- manual investigation hours per unresolved history;
- number of decisions that remain unresolved because evidence is insufficient.

Do not publish only an aggregate coverage percentage. Publish the complete coverage vector, critical-source results and the declared scope.

### 7.3 Statistical claims

Repeated events from one source, sensor or article are not independent experimental units. Report the true dependence hierarchy. For physical studies, use article/source/lot/operator/time-block holdouts as appropriate, and report confidence intervals for omission detection and false completeness. The synthetic 100% gates above must not be presented as a physical 100% reliability claim.

## 8. Temporal-database and evidence-replay tournament

The existing temporal scale harness is a useful smoke benchmark, but it measures the in-memory Rust evaluator over a synthetic `serde_json::Value` event array. It does not measure durable persistence, receiver ingestion, independent source reconciliation, cryptographic verification, long-term retention, or NDE. Its 10-million-event tier should therefore be treated as a stress target subject to a declared memory/resource budget, not a release gate.

The next tournament should freeze the same workload, resource envelope, data distribution and independent expected outputs across these arms:

1. **Latest-row/current-state baseline:** unsafe negative control for overwritten history and backdating.
2. **Append-only temporal log:** event/valid/transaction time with source offsets and correction history.
3. **Bitemporal relational baseline:** valid-time and transaction-time history with a separately versioned trust view.
4. **Single-watermark evidence replay:** current AERO-style subject/evidence/trust model using one receiver cutoff.
5. **Vector-watermark replay:** source/partition-scoped receiver inclusion vector and causal dependencies.
6. **Vector-watermark plus qualified coverage:** complete AERO research target with a BASALT-owned coverage receipt and independent reconciliation.

NIST IR 8536 is an interoperability/traceability comparator, not a temporal database implementation to copy. W3C PROV is a provenance interchange comparator. RFC 3161 and RFC 4998 are timestamp and long-term evidence preservation comparators. None alone establishes physical event truth or observer completeness.

### 8.1 Workloads

- 10^4 events: functional and mutation smoke.
- 10^5 events: multiple sources, partitions, corrections, gaps and trust changes.
- 10^6 events: replay, causal-cut and incremental-reassessment workload.
- 10^7 events: retention/stress experiment only if the declared hardware budget supports it.

Use fixed-seed distributions for late arrivals, clock skew, source count, partition count, event size, duplicates, correction depth, sequence gaps, forks, observer outages, trust changes and checkpoint frequency.

Separate at least three storage/evaluation paths:
- compact typed in-memory representation;
- durable indexed store with ingestion and query costs;
- streaming/partitioned history with bounded working memory.

A JSON object graph benchmark must not be presented as the throughput of a production evidence store.

### 8.2 Performance and cost measurements

Report p50/p95/p99 latency, sustained ingest throughput, CPU time, peak resident memory, index/storage amplification, bytes per event, checkpoint creation cost, cold-start and recovery time, incremental-replay ratio, full rebuild cost, and cost under concurrent receipt/revocation updates.

For each arm, report correctness first:
- exact historical replay agreement;
- future-knowledge bleed;
- evidence omissions and false completeness;
- correct abstention/unknown results;
- reproducibility under input permutation and equivalent partition ordering.

A faster arm that accepts an incomplete history is disqualified, not ranked as a performance win.

Prospective stretch targets, only at matched semantics and correctness:
- at least 2x lower p95 replay latency; or
- at least 30% lower retained-index, storage or compute cost.

These are research targets, not achieved results.

## 9. Physical SHM / NDE connection

Observer coverage is a direct bridge between temporal replay and physical sensing. The Utah long-term guided-wave dataset is especially useful for robustness research because it spans over 4.5 years, includes approximately 6.4 million guided-wave observations, environmental changes, sensor drift, installation shifts and 13 introduced damage types. Its source files include Python pickle containers; the existing AERO ingestion boundary prohibiting untrusted deserialization on workstations, shared CI and production remains in force.

NASA's CFRP PCoE data provides a different comparison: fatigue-run observations, guided-wave sensor networks and periodic X-ray characterization. These datasets can support research on source/channel observability and sensing integrity, but they do not automatically establish a production observer-coverage qualification.

For any physical study:
- define the physical article and independent experimental unit;
- preserve raw signals and source acquisition metadata;
- preregister source/channel loss injections and damage/control conditions;
- distinguish sensor failure from structural damage and environmental variation;
- independently determine injected/known events and capture failures;
- report method-specific POD only with an appropriate physical NDE study;
- never use a coverage score to replace an approved inspection or maintenance requirement.

## 10. Ownership and integration

| Capability | Owner | AERO responsibility |
|---|---|---|
| Source bytes, artifact identity and receiver checkpoint | Mycelix/AeroCommons | Consume exact references and digests |
| Generic observer coverage qualification | Symthaea BASALT | Consume the qualified coverage receipt; define AERO challenge inputs |
| Generic freshness/invalidation and lifecycle | BASALT / ASSURE | Preserve owner-native status and history |
| Measurement uncertainty and sample dependence | Symthaea uncertainty provenance | Consume qualified uncertainty evidence |
| Engineering acquisition/effectivity semantics | Symtropy AERO | Bind coverage to physical interval and engineering subject |
| Physical truth, NDE procedure and test acceptance | Qualified laboratory / authority | Preserve their evidence and claim ceiling |

No AERO code may turn a structurally valid receipt into a trusted observer qualification unless BASALT's owner-native qualification evidence verifies that claim.

## 11. Implementation sequence and exit criteria

**Stage 0 — specification and review**
- Review this contract and the 20+ mutation families independently.
- Agree on the scope-manifest owner and exact BASALT receipt reference.
- Freeze the vector-watermark representation and disposition semantics.

**Stage 1 — synthetic adapter contract**
- Add a Draft 2020-12 schema and frozen synthetic corpus.
- Add a separate verifier test that checks scope, source/partition watermarks, sequence continuity, receipt/checkpoint binding and non-compensatory critical channels.
- Keep oracle status `NOT_INDEPENDENTLY_REVIEWED` until an independent reviewer signs off.

**Stage 2 — owner-native integration**
- Consume real BASALT-qualified observer receipts and Mycelix checkpoints.
- Validate receipt signatures, digests, source identities and checkpoint binding through their owning verifiers.
- Test stale, missing, incomplete, revoked and mismatched receipts.
- Keep the integration non-authorizing: it may qualify a replay input, not release an aircraft or material.

**Stage 3 — physical pilot**
- Rights-clear the dataset or authorize collection.
- Preregister instrumentation, fault injection, ground truth, power/precision and stop rules.
- Demonstrate omission detection and raw-evidence preservation under controlled faults.
- Obtain independent laboratory and engineering review.

**Exit gates**
- independent oracle review complete;
- all frozen synthetic challenges produce the expected disposition;
- all incomplete, unverified, invalid and out-of-scope negative controls fail closed;
- exact receipt/checkpoint/scope/observer generations are bound;
- replay is deterministic for an identical coordinate and verifier build;
- physical and synthetic claims remain separated;
- measured resource/cost results have matched baselines and reproducible manifests.

## 12. Claim boundary

This specification and its synthetic fixtures do not prove cryptographic receipt validity, trusted time, complete source discovery, physical measurement truth, metrological traceability, NDE probability of detection, material qualification, structural substantiation, airworthiness or certification.

The intended contribution is narrower and testable:

**AERO should only make a negative engineering inference when the exact historical evidence view is integrity-bound, the trust state is pinned, and the observation boundary has been independently qualified for the relevant subject, sources, channels and interval.**

## References

- NIST IR 8536, *Supply Chain Traceability: Manufacturing Meta-Framework* (final, 9 September 2026): https://csrc.nist.gov/pubs/ir/8536/final
- W3C PROV-DM: https://www.w3.org/TR/prov-dm/
- RFC 3161, Time-Stamp Protocol: https://www.rfc-editor.org/rfc/rfc3161
- RFC 4998, Evidence Record Syntax: https://www.rfc-editor.org/rfc/rfc4998
- Kaufmann et al., *Benchmarking Bitemporal Database Systems* (EDBT 2014): https://websci.informatik.uni-freiburg.de/publications/EDBT2014-Benchmarking-Bitemporal-Database-Systems.pdf
- NASA PCoE Data Set Repository: https://www.nasa.gov/intelligent-systems-division/discovery-and-systems-health/pcoe/pcoe-data-set-repository/
- *Dataset on guided waves from long-term structural health monitoring under uncontrolled and dynamic conditions*, Scientific Data (12 June 2025): https://www.nature.com/articles/s41597-025-05300-5
- ASTM E2862-23, hit/miss POD analysis: https://store.astm.org/e2862-23.html
