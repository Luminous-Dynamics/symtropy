# AERO Causal Evidence-Cut Contract and Benchmark v0.1

**Related:** AERO temporal assurance #7455 / PR #7456; observer coverage PR #7508; BASALT observer qualification; Mycelix/AeroCommons evidence receipts  
**Status:** research-only contract and benchmark protocol  
**Claim ceiling:** typed synthetic source-watermark and causal-cut consistency checks  
**Oracle status:** implementation tests are authored with the reference module and are not an independent oracle  
**Physical status:** no source capture, cryptographic verification, observer qualification or physical engineering decision is implemented

## 1. Why this increment exists

A single receiver timestamp does not define a distributed evidence view. Independent instruments, calibration authorities, test systems and manufacturing sources may have different partitions, sequence spaces, stream resets, receiver offsets and causal dependencies.

A global maximum timestamp can hide at least four failures:
- a source partition has not reached the event sequence required by a dependent observation;
- a stream has restarted and sequence numbers have been reused under a new epoch;
- a dependent event is present while its causal predecessor is absent;
- two records claim the same source/partition/epoch/sequence.

The vector-watermark model therefore treats evidence inclusion as a **source- and partition-scoped cut**. It does not claim that wall-clock time, a database transaction timestamp or a maximum offset orders every event across all sources.

The research Rust module is:

`crates/domains/symtropy-aero-material-qual/src/vector_watermark_v0_1.rs`

It checks internal consistency and causal closure of an already declared evidence cut. It does not discover the complete source universe, authenticate checkpoint digests, verify real source receipts, or establish that a sensor or observer captured every physical event. Those are upstream proof obligations owned by BASALT, Mycelix/AeroCommons and the relevant source owners.

## 2. Typed evidence-cut representation

The cut comprises two related collections.

### 2.1 Partition watermark

Each source partition has:

| Field | Meaning |
|---|---|
| `source_id` | Stable source identity, not a caller-selected alias |
| `partition_id` | Source-native partition or sequence namespace |
| `stream_epoch` | Authenticated generation of the stream; resets create a new epoch |
| `last_contiguous_sequence` | Greatest sequence for which every required predecessor through that sequence is accounted for |
| `receiver_inclusion_offset` | Receiver-native durable inclusion offset |
| `checkpoint_digest_sha256` | Digest reference for the owner-native checkpoint |

A watermark is not a statement that the source roster is complete. It is not a cryptographic proof merely because a digest-shaped string is present. A production adapter must verify the source checkpoint and receipt through their owner-native verifier.

### 2.2 Included event and causal dependencies

Each included event carries:
- immutable event reference;
- source, partition and stream epoch;
- source sequence;
- receiver inclusion offset;
- explicit dependencies on required predecessor events.

The event identity is not reduced to a display label. A dependency binds its event reference to the exact source, partition, epoch and sequence. If those coordinates disagree with the referenced event, the cut is invalid.

### 2.3 Causal-cut rule

A declared event may be included only when:
1. its source/partition has a watermark;
2. the event epoch matches the watermark epoch;
3. its sequence is no greater than the last contiguous sequence;
4. its receiver offset is no greater than the checkpointed receiver inclusion offset;
5. every declared causal predecessor is present;
6. each predecessor coordinate matches its referenced event;
7. the predecessor is itself covered by its source/partition watermark;
8. the explicit dependency graph is acyclic.

Concurrent events on independent partitions are allowed to remain incomparable. The validator must not invent a total order to make a replay deterministic.

A scalar receipt watermark can still be recorded as metadata, but it must not replace this partition vector.

## 3. Dispositions and precedence

| Disposition | Meaning | Replay treatment |
|---|---|---|
| `CAUSALLY_CLOSED` | Every declared event and explicit causal predecessor is covered by the vector | The cut is structurally usable, subject to upstream trust and coverage checks |
| `INCOMPLETE` | A predecessor is absent, a predecessor is beyond the contiguous watermark, or a prior partition disappeared | Abstain |
| `UNVERIFIED` | Watermark or authenticated epoch relationship is missing | Trust unresolved |
| `INVALID` | Duplicate partition watermark, malformed digest, contradictory event inclusion, source-sequence fork, dependency coordinate mismatch or causal cycle | Reject |

Precedence is deliberately conservative:
1. Integrity/contradiction failures yield `INVALID`.
2. Missing trust/epoch information yields `UNVERIFIED`.
3. Known missing evidence yields `INCOMPLETE`.
4. Only a structurally closed graph yields `CAUSALLY_CLOSED`.

This precedence does not allow a known missing predecessor to be hidden by a healthy aggregate completeness score.

## 4. Watermark progression

When comparing a new checkpoint with its predecessor:
- every previously present source/partition remains represented unless the owner supplies an explicit partition retirement/change contract;
- sequence and receiver inclusion offsets cannot move backwards within the same stream epoch;
- a stream epoch change requires a separately authenticated transition record;
- a new partition may be added, but its introduction must be represented in the source-universe manifest and coverage qualification;
- same-epoch sequence reuse or fork is invalid.

The research helper currently treats an unproved epoch change as `UNVERIFIED`, a removed previous partition as `INCOMPLETE`, and a backwards sequence/offset as `INVALID`. It does not yet consume a real epoch-transition receipt or partition-retirement contract.

## 5. Adversarial contract tests

The Rust reference module includes controls for:
- two independent concurrent source partitions;
- an absent causal predecessor;
- a stream epoch mismatch;
- an event asserted included beyond the checkpoint watermark;
- duplicate source sequence / fork;
- a causal dependency cycle;
- watermark sequence regression;
- stream epoch change without transition evidence;
- malformed checkpoint digest.

The tests establish only deterministic behaviour of the synthetic module. They do not count as an independent oracle because the tests and implementation are maintained together. Before a confirmatory benchmark, expected dispositions must be independently derived, frozen and reviewed.

## 6. Benchmark tournament

Compare at least these arms on the same synthetic history and hardware budget:

1. **Latest-row/current-state baseline:** intentionally unsafe negative control.
2. **Scalar wall-clock cutoff:** includes records whose asserted time is no later than a single cutoff.
3. **Source-vector inclusion without causal closure:** tracks each partition but does not validate dependency closure.
4. **Source-vector plus causal closure:** the current research Rust design.
5. **Append-only event-sourced temporal baseline:** event/valid/transaction-time histories and correction lineage.
6. **Bitemporal relational baseline plus separately versioned trust snapshot.**
7. **Full AERO evidence replay:** acquisition interval, evidence checkpoint vector, independently qualified observer coverage, trust snapshot and immutable report lineage.

NIST IR 8536 is an interoperability/traceability comparator rather than a causal-cut implementation to clone. W3C PROV is a provenance interchange comparator. ISO 23247 is a manufacturing digital-twin framework and does not prescribe one data format or protocol. AERO should expose mappings/adapters rather than claim to replace these standards.

### 6.1 Workload design

Use fixed-seed histories with controlled:
- number of sources, partitions and stream epochs;
- events per partition and distribution skew;
- independent versus causally dependent events;
- dependency fan-in and chain depth;
- late arrivals and receiver offset delay;
- event-effective-time uncertainty intervals versus physical acquisition-time intervals, including exact-point equality and ambiguous overlap;
- missing predecessors, sequence gaps and forks;
- partition retirement and stream reset;
- duplicate delivery;
- trust policy and revocation changes;
- checkpoint frequency and observer outages.

Scale tiers:
- 10^4 events: functional smoke and mutation suite;
- 10^5 events: multi-source and dependency-DAG workloads;
- 10^6 events: replay, correction and concurrent reassessment;
- 10^7 events: stress/retention experiment only under a declared memory and hardware envelope.

### 6.2 Measures

Report:
- exact result agreement with the independently authored oracle;
- false causal-closure acceptance;
- missing-predecessor and fork detection;
- future-knowledge bleed;
- event and dependency counts;
- p50, p95 and p99 validation latency;
- sustained ingestion and checkpoint throughput;
- peak resident memory and bytes per event/dependency;
- index/storage amplification;
- incremental recomputation ratio and full-rebuild cost;
- cold-start and recovery time;
- sensitivity to partition count, dependency fan-in and late-arrival skew.

A faster result is disqualified if it accepts a causally open cut or changes the historical result.

Prospective stretch targets at matched correctness and resources:
- >=2x lower p95 validation/replay latency; or
- >=30% lower retained-index/compute cost.

These are targets, not achieved results. The current module has not yet been benchmarked against a production-grade stream processor or temporal database.

## 7. Public physical SHM comparison

The University of Utah long-term guided-wave dataset is a suitable public research candidate for the physical-observation side. It covers more than 4.5 years, approximately 6.4 million measurement instances, 13 introduced damage types, environmental variation, sensor drift and installation shifts. It should be used for robustness and observability experiments only after checking the data's rights and acquisition format. Its public data may contain serialization formats that must be treated as untrusted input.

NASA's Prognostics Center of Excellence repository provides further prognostic time-series datasets and asks users to acknowledge dataset donors. It is a repository of research datasets, not a production qualification authority.

For a physical benchmark, define article/sensor/time dependence and split by physical unit, sensor generation or acquisition period. Randomly splitting adjacent signal windows can create leakage. Do not report universal POD: ASTM E2862-23 applies to a defined hit/miss NDE examination configuration and known discontinuity-size range, with method assumptions that must be satisfied.

## 8. Standards and interoperability boundary

| Reference | AERO use |
|---|---|
| NIST IR 8536 | Interoperable provenance, traceability, secure links and selective disclosure |
| W3C PROV-DM | Provenance vocabulary for entities, activities and agents |
| ISO 23247 series | Manufacturing digital-twin framework and observable manufacturing elements |
| RFC 3161 | Independent timestamp evidence over data digests |
| RFC 4998 | Long-term evidence preservation and timestamp renewal |
| ASME V&V 10 / VVUQ 10.2 | Computational mechanics credibility and uncertainty context |
| ASTM E2862-23 / MIL-HDBK-1823A | Applicable hit/miss POD analysis, not a generic assurance score |
| FAA 14 CFR 25.571 / AC 25.571-1D | Structural damage-tolerance and fatigue evaluation; AERO does not confer inspection credit |

## 9. Engineering economics

Measure the total cost of reaching a risk-acceptable conclusion:
- engineering and quality hours;
- laboratory and NDE hours;
- repeated/duplicated inspection;
- time waiting for missing supplier/source evidence;
- false acceptance and false rejection cost;
- downtime and inspection exposure;
- evidence collection, retention and verification overhead;
- replay and reassessment cost;
- unresolved-decision rate and time to resolution.

The optimization target is not simply fewer tests or faster inference. It is lower cost and elapsed time **at matched or better risk control**, with mandatory inspection and certification obligations preserved.

## 10. Claim boundary and promotion gates

This module is a research-only structural validator. It does not authenticate checkpoint digests, validate cryptographic signatures, establish trusted time, verify source receipts, prove source-universe completeness, qualify an observer, validate NDE/POD, establish material conformity or make airworthiness decisions.

Promotion requires:
1. independent review of the causal-cut contract and oracle;
2. fuzz/property-based testing of dependency graphs, stream epochs and vector progress;
3. owner-native checkpoint, receipt and epoch-transition verification;
4. integration with BASALT-qualified observer coverage;
5. matched baseline benchmark runs with reproducible manifests;
6. physical pilot preregistration and independent laboratory review where physical claims are sought.

## References

- NIST IR 8536, *Supply Chain Traceability: Manufacturing Meta-Framework* (final, 9 September 2026): https://csrc.nist.gov/pubs/ir/8536/final
- W3C PROV-DM: https://www.w3.org/TR/prov-dm/
- ISO 23247-1:2021: https://www.iso.org/standard/75066.html
- RFC 3161, Time-Stamp Protocol: https://www.rfc-editor.org/rfc/rfc3161
- RFC 4998, Evidence Record Syntax: https://www.rfc-editor.org/rfc/rfc4998
- ASME V&V 10–2019 (R2025): https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019
- ASTM E2862-23: https://store.astm.org/e2862-23.html
- NASA Prognostics Center of Excellence Data Set Repository: https://www.nasa.gov/intelligent-systems-division/discovery-and-systems-health/pcoe/pcoe-data-set-repository/
- University of Utah long-term guided-wave dataset (Scientific Data, 12 June 2025): https://www.nature.com/articles/s41597-025-05300-5
- 14 CFR §25.571 and FAA AC 25.571-1D: https://www.ecfr.gov/current/title-14/section-25.571 ; https://www.faa.gov/regulations_policies/advisory_circulars/index.cfm/go/document.information/documentid/865446
