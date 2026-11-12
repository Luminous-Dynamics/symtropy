# Symtropy Maritime Observability Adapter

A thin maritime asset/sensor adapter over `symtropy-sensor-observability-bench`. Pairwise interval-separation remains in the general crate; this adapter owns maritime identity mapping, sensor dependency admission, scenario replay and report translation.

## Run

```sh
cargo test --manifest-path crates/domains/symtropy-maritime-observability-bench/Cargo.toml
cargo run --manifest-path crates/domains/symtropy-maritime-observability-bench/Cargo.toml -- benchmarks/maritime-observability/maritime-observability-case-v0.1.json --source-commit <40-hex> --case-sha256 <64-hex> --clock-qualification benchmarks/maritime-observability/synthetic-clock-qualification-v0.1.json --clock-sha256 <64-hex>
```

## P0.2 semantics

- Binds vessel/engine/configuration identity, operating regime, sensor topology and a versioned fault taxonomy.
- Maps maritime measurement-principle identities to the generic evaluator's independent-principle groups.
- Retains common-cause dependencies separately; shared gateway/clock/power loss is replayed as an explicit outage scenario.
- Stale/unknown quality, expired/unknown calibration, derived/reconstructed values and unsupported source roles cannot contribute independent measured evidence.
- Replays every declared sensor/common-cause loss scenario through the generic evaluator.
- Emits a read-only, source/case-digest-bound report.

The adapter derives structural detectability against the declared reference state and pairwise fault isolability from the complete, synchronized response matrix. It does not measure empirical event-detection accuracy, train a classifier, infer unique root causes where signatures remain ambiguous, qualify physical sensor installations or authorize actuation/maintenance/mode transitions. Synthetic fixture metrics are structural only.


## P0.2: fault-level ambiguity report

The case declares `layout_profile.reference_state_id` explicitly. The adapter derives per-fault states from the complete pair matrix: `observable_and_isolable`, `ambiguity_class`, `unobservable`, `unsupported_regime` or `not_evaluated`. A deterministic ambiguity class is emitted when two or more faults are independently detectable against the reference but pairwise indistinguishable. The output retains candidate fault IDs and source evidence references. Unknown JSON fields, duplicate response IDs and incomplete pair matrices are rejected.

Response-level `evidence_role` is now authoritative alongside sensor source role, calibration and quality. A response labelled MODEL_ESTIMATE or OPERATOR_REPORTED cannot be promoted to an independently measured/synthetic signature merely because the sensor inventory says measured/synthetic. Such a response is retained as unknown/non-admissible evidence.


## P0.3: common-timebase qualification

Pairwise response intervals are admitted only after sensor timing is bound to the case's declared common measurement timebase. Each installed sensor records clock domain, mapped timebase ID, timestamp uncertainty and its source, qualification state, and the exact qualification artifact ID/SHA-256. Both the case and layout profile set a maximum timestamp uncertainty.

For a measured case, the adapter requires a qualified timebase and uncertainty sourced from a measured offset bound or instrument/vendor specification. For this synthetic conformance corpus only, a separately hashed synthetic clock-qualification fixture is accepted and remains synthetic-only. Unknown/estimated clock evidence, unmapped timebases, excessive uncertainty or an unqualified operating domain make the associated response cells UNKNOWN; they cannot support diagnosability.

The workflow hashes the synthetic clock fixture independently and checks its digest against every sensor record. This proves fixture-byte linkage, not clock accuracy. The adapter does not implement PTP or authenticate physical clock measurements; a real installation still needs independent offset, drift, path-asymmetry and end-to-end timestamp/latency evidence. See [ISO 13379-1:2025](https://www.iso.org/standard/88027.html) and the [2026 industrial time-synchronization review](https://doi.org/10.1109/MIAS.2026.3682180).

## Qualification semantics

`observable_and_isolable` means a fault is distinguishable from normal and every declared in-domain alternative. `ambiguity_class` is reserved for a connected set of at least two faults that are each detectable against normal but remain pairwise indistinguishable. A fault detectable versus normal but not uniquely isolable from a fault outside such a connected set is reported as `detectable_not_isolable`, rather than receiving an empty or fabricated ambiguity class.


## P0.4 — exact timebase-artifact reconciliation and non-transitive ambiguity

The CLI requires the separate clock-qualification JSON and its byte-level SHA-256. The evaluator checks the artifact schema version, evidence class, qualification ID, timebase ID, clock domain, mapping method, uncertainty source, declared uncertainty, layout maximum and hardware-measurement flag against every installed sensor. The operating-domain, layout-profile and clock-artifact maxima must agree. For this synthetic identity-mapping fixture, every sensor must belong to the one clock domain named by the fixture. A mismatch withdraws the case before pairwise reporting.

Fault ambiguity is represented by maximal pairwise-indistinguishable sets (maximal cliques), not graph connected components. Interval overlap is not transitive: A can overlap B, B overlap C, while A and C remain disjoint. A fault may belong to more than one ambiguity set; the report carries all `ambiguity_class_ids` and an explicit top-level list of ambiguity classes and pairwise non-isolable pairs. It does not collapse a chain into a false equivalence class.

The embedded sensor-topology, fault-model, evaluator, dataset, split and model SHA fields are null in P0 because no separate artifact bytes are supplied for those objects. The execution receipt binds the exact case-file SHA-256 and exact source commit; the clock artifact receives its own separately computed byte digest.


### Per-sensor clock qualification and graceful degradation

The common timebase artifact and each sensor's clock qualification are separate evidence objects. An installed sensor may be present but have UNQUALIFIED/UNKNOWN timebase state; it remains in the report and unqualified-sensor count, but its responses are treated as UNKNOWN and cannot contribute to pairwise physical evidence. Missing per-sensor clock artifact ID/SHA and timestamp-uncertainty values are permitted only for those unqualified states.

A qualified sensor must bind the exact common timebase artifact ID/SHA and declared clock domain. Its uncertainty bound may not understate the common artifact's declared uncertainty. An uncertainty bound above the layout maximum withdraws that sensor's evidence rather than invalidating the whole assessment. This preserves valid independent channels when one sensor's clock is not qualified.


## P0.5 — exact-head validation

The exact adapter head `98a35a3c85f5318eacae854849ef5f1fe655a521` passed both focused adapter workflows (43162088096 and 43162080730): 21/21 Rust unit tests, strict Clippy, frozen-case execution and receipt checks. The generated report has eleven fault pairs: eight isolable, two non-isolable and one not evaluated. A shared gateway loss removes all eight base-isolable distinctions; loss of the fuel-pressure transducer alone removes none because other independent principles remain. This is synthetic conformance only.

The case's separate sensor-topology/evaluator/data/split/model digest slots remain null, because those exact artifact bytes were not supplied. The execution receipt binds only the exact commit, frozen case bytes, and synthetic clock artifact bytes.

## P0.6 — reviewed dependency topology and shared-failure coverage

Every installed sensor now declares a dependency-review state. Synthetic cases may claim only `synthetic_conformance`; they cannot label a fabricated dependency graph independently reviewed. Measured cases may contribute physical evidence only when the dependency graph has an independently reviewed record ID and SHA-256. Unreviewed/unknown dependency state preserves the sensor in the report but excludes it from physical evidence.

Every installed sensor must declare at least one dependency group. The validator counts group membership and requires an explicit outage scenario for every dependency group shared by two or more installed sensors. The synthetic fixture now replays four scenarios: gateway loss, clock loss, combined gateway-plus-clock loss, and individual fuel-pressure sensor loss. These are fabricated topology/outage controls; they do not establish the real engine's acquisition architecture.


## Measurement-window coherence

All response channels within one fault-state hypothesis must refer to the same declared synchronized measurement window. The adapter rejects a matrix that combines different windows for different channels of the same state; pairwise interval separation is meaningless if the compared channels do not describe the same operating episode. The window identity is distinct across state hypotheses, so the exact comparison windows remain auditable in the source response manifest.


## P0.8 — guaranteed common acquisition-window overlap

The previous P0.2 check required all sensor responses for a state to carry the same `measurement_window_ref`. That identifier equality alone does not establish that acquisition intervals overlap in time.

P0.8 adds explicit `window_start_ms` and `window_end_ms` to every response, plus a positive `minimum_common_window_overlap_ms` to the layout profile. For each admitted channel, the evaluator computes a conservative guaranteed interval using that sensor's qualified timestamp uncertainty:

- guaranteed start = recorded start + timestamp uncertainty;
- guaranteed end = recorded end − timestamp uncertainty.

A state window is qualified only when the intersection across contributing channels is at least the declared minimum duration. Otherwise, the affected state is withdrawn from pairwise diagnosis rather than combining asynchronous response signatures. The report exposes the guaranteed common interval and overlap duration.

The same calculation is repeated after every declared sensor/common-cause outage. Removing an unaligned sensor may restore coherent evidence from the remaining channels; losing a shared clock can withdraw the window and its diagnoses. The fixture is synthetic; its timestamp and window values do not demonstrate shipboard synchronization.


## P0.9 — exact diagnosability resilience and minimal cut sets

The adapter enumerates every singleton installed-sensor outage and every pairwise combination of those sensor outages and shared dependency-group outages. Dependency groups shared by two or more installed sensors are common-cause atoms; singleton dependencies are subsumed by their sensor atom to avoid duplicate labels for the same sensor loss.

For the frozen fixture this is 7 failure atoms (5 sensors + 2 shared dependencies) and 28 exact outage combinations at cut order 2. For each baseline-identifiable critical fault pair, the evaluator reports every inclusion-minimal cut set that makes the pair non-identifiable or not evaluated, the minimum cut order found, and the number of enumerated failure atoms tolerated before the first cut. Baseline non-identifiable pairs are marked `baseline_not_qualified`; no positive resilience is assigned to them. No-cut results through the bound report only a lower bound.

The search is exact and bounded. Inputs exceeding the declared bounds fail closed; there is no random sampling or heuristic fallback. Failure atoms are outage hypotheses and are **not** assumed probabilistically independent. The cut sets are structural evidence for the exact declared topology and response matrix, not field reliability probabilities.
