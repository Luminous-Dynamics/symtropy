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

The adapter does not compute individual-fault detectability, train a classifier, infer root causes, qualify physical sensor installations or authorize actuation/maintenance/mode transitions. Synthetic fixture metrics are structural only.


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
