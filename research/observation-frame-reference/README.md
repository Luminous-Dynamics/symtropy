# Observation-frame reference oracle — V0.1

A Bevy-free Rust reference implementation for the finite immutable observation-frame contract owned by Symtropy issue #4468.

This package is deliberately not a production observation authority or an engine adapter. It does not read a Bevy world, query physics, authenticate upstream sources, measure hardware, qualify sensors, assess model credibility, or authorize actuation. Its schema is a test/reference representation that must be reviewed against #4468 before any production DTO adoption.

## Run

From this directory:

    cargo test --locked
    cargo clippy --locked --all-targets -- -D warnings
    cargo run --locked -- fixtures/machine-test-01.json
    cargo run --release --locked --bin benchmark -- 1000

The package is isolated from the Bevy workspace and pins direct dependencies. Its lockfile is extracted from the same resolved dependency versions used by the isolated world-experiment-lease research package; Cargo --locked checks completeness in CI.

## Reference contract

The observable-set profile digest is recomputed from the canonical set of (ScenarioEntityId, coordinate frame, observable ID, quantity ID, unit) entries and checked against the input identity. It excludes measured values and source classes: a channel remains requested even when its value is unavailable. A view that needs additional channels must not query the live physics world after this frame; it requires a new declared observation frame. The input binds:
- scenario/run/source revision, branch/replay, initial/current state and model profile identities;
- fixed simulation tick and exact rational model-time clock;
- stable ScenarioEntityId and coordinate-frame identity;
- claim-bearing flag, explicit runtime subject mapping and binding receipt;
- typed observable ID, quantity, unit, source class and exact value encoding;
- optional source clock/time, provenance and uncertainty;
- optional ordered source event stream.

Entities and observables are semantic sets, canonicalized by stable IDs. SHA-256 hashes are computed by streaming serde_json serialization directly into the hasher, avoiding a second frame-sized JSON byte buffer. The source event stream is an ordered sequence and is never sorted. If absent, the receipt leaves event-order identity absent and emits EventOrderUnavailable.

Binary64 values are represented as exactly 16 lowercase hexadecimal digits of IEEE-754 bits. This preserves signed zero and NaN payloads rather than normalizing through JSON decimal numbers. Non-finite values remain visible in the receipt findings; malformed or reversed uncertainty intervals reject.

Unavailable observations require a reason and no value. PhysicalMeasurement and CalibratedMeasurement require a source digest. Numeric zero is an ordinary explicit value, never a substitute for missing data.

## Bounded inputs

The CLI rejects files over 64 MiB before JSON decoding. The reference validator caps entity count, observables per entity, total observables, event count, and text/value lengths. These are prototype safety bounds for this reference package, not universal product limits.

## Benchmarks

The in-tree std::time benchmark binary (no Criterion or other benchmark-specific dependency) generates deterministic 1k, 10k and 100k claim-bearing entity frames with 1, 4 and 1 observables/entity respectively, plus ordered event streams. It reports two warm-up passes followed by 1,000 measured samples and nearest-rank p50/p95/p99 valid compile latency in nanoseconds, min/max, and p50/p95/p99 deep-rejection latency for a well-formed frame with a mismatched observable-set commitment. It also records pinned rustc version, CPU model, GitHub runner image identity, serialized input bytes, total input bytes per entity (including the ordered event stream), validated entities/second and event records/second. The workflow preserves the raw JSONL measurements as an artifact. These are reference-oracle costs, not Bevy extraction latency or physics/rendering throughput. With 1,000 samples, the empirical p99 uses ten upper-tail observations; always compare results only with matching source head, Rust compiler version, CPU model/architecture, OS and runner-image version.

## Tests

The 47-test corpus covers entity/observable order invariance, ordered-event sensitivity, absent event identity, source-class preservation, unavailable-versus-zero semantics, observable-set and full-frame known-answer digests, membership mismatch, value-independent channel identity, unavailable-channel retention, malformed values/uncertainty, duplicate identities, stable NetId generation scope, borrowed-view lifetime rejection, non-finite IEEE values, presentation-field rejection and source-clock round-trip.

## Claim ceiling

Passing tests establish only structural validation and deterministic canonical identity of this supplied reference representation for its finite fixtures. They establish no physical truth, observation authenticity, source freshness, sensor calibration, numerical health, model credibility, rendering correctness, engineering acceptance, safety, certification, or actuation authority.


## Borrowed-view generation fence

The reference package includes a semantic guard for optional zero-copy/read-only live views. Eight focused tests exercise exact-match acceptance, world replacement, generation change, rebuild epoch, frame advance, observable-set change, malformed token and owned-snapshot persistence. The benchmark also varies channel density at the 10k-entity point, measures valid guard and rebuild-rejection p50/p95/p99 separately from frame compilation, and reports a third owned-reference-DTO clone cost. The DTO clone is only a serialization/data-structure cost comparator; it is not a measurement of Bevy world extraction or real sensor-buffer copying. A borrowed view is accepted only while world-instance ID, world-generation ID, rebuild epoch, frame digest and observable-set digest all match the current buffer identity. It rejects world replacement, generation change, model rebuild, frame advance and channel-profile change. An owned observation receipt remains a historical value and is not invalidated by later world rebuilds.

This guard is not a memory-safety primitive and does not extend a buffer's lifetime. A production adapter must use safe Rust ownership/borrowing or an independently qualified runtime handle, and call the guard before each boundary where the source may have rebuilt or advanced. The guard alone cannot prevent a concurrent rebuild between validation and dereference; production integration must serialize access or otherwise make the check-and-use atomic with respect to rebuild.
